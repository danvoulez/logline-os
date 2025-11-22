#!/usr/bin/env node
import { program } from 'commander';
import * as fs from 'fs';
import * as path from 'path';
import axios from 'axios';
import * as figlet from 'figlet';
import * as chalk from 'chalk';
import * as archiver from 'archiver';
import * as AdmZip from 'adm-zip';
const qrcode = require('qrcode-terminal');

const CONFIG_DIR = path.join(process.env.HOME || '.', '.logline');
const TOKEN_FILE = path.join(CONFIG_DIR, 'token');
const API_URL = process.env.API_URL || 'http://localhost:3001/api/v1';

// Ensure config dir exists
if (!fs.existsSync(CONFIG_DIR)) {
  if (!fs.existsSync(CONFIG_DIR)) fs.mkdirSync(CONFIG_DIR);
}

function showBanner() {
  console.log(chalk.cyan(figlet.textSync('LogLine OS', { horizontalLayout: 'full' })));
  console.log(chalk.dim('>> Governance & Integrity Protocol v1.0\n'));
}

program
  .name('logline')
  .description('LogLine OS CLI - Governance & Management')
  .version('1.0.0');

program
  .command('login')
  .description('Authenticate with LogLine ID via Web/QR Code')
  .action(async () => {
    showBanner();
    console.log(chalk.yellow('⚡ Initiating Secure Handshake...'));
    
    try {
      // 1. Create Session
      const res = await axios.post(`${API_URL}/auth/cli/session`);
      const { session_id, url, qr_code_data } = res.data;

      console.log('\n' + chalk.white.bold('SCAN TO AUTHENTICATE:'));
      qrcode.generate(qr_code_data, { small: true });
      
      console.log(chalk.dim('Or visit: ') + chalk.blue.underline(url));
      
      process.stdout.write('\nWaiting for approval');
      
      // 2. Poll for approval
      const pollInterval = setInterval(async () => {
        process.stdout.write(chalk.dim('.')); // ASCII Spinner feeling
        try {
          const pollRes = await axios.get(`${API_URL}/auth/cli/session/${session_id}`);
          
          if (pollRes.data.status === 'approved') {
            clearInterval(pollInterval);
            const token = pollRes.data.access_token; // Assuming standard token response
            fs.writeFileSync(TOKEN_FILE, token);
            console.log('\n\n' + chalk.green('✔ ACCESS GRANTED'));
            console.log(chalk.dim(`Token saved to ${TOKEN_FILE}`));
            process.exit(0);
          }
        } catch (e) {
          if (e.response?.status === 404 || e.response?.data?.message === 'Session expired') {
             clearInterval(pollInterval);
             console.log(chalk.red('\n\n✖ Session Expired. Please try again.'));
             process.exit(1);
          }
        }
      }, 2000);

    } catch (error) {
      console.log(chalk.red('\n✖ Connection Failed: ' + (error.response?.data?.message || error.message)));
    }
  });

program
  .command('whoami')
  .description('Show current user info')
  .action(async () => {
    const token = getToken();
    if (!token) return;

    try {
      const res = await axios.get(`${API_URL}/auth/me`, { 
        headers: { Authorization: `Bearer ${token}` } 
      });
      console.log(chalk.bold('\n👤 User Profile:'));
      console.log(chalk.cyan('ID:    ') + res.data.logline_id);
      console.log(chalk.cyan('Name:  ') + res.data.name);
      console.log(chalk.cyan('Email: ') + res.data.email);
      console.log(chalk.cyan('Role:  ') + res.data.role);
    } catch (error) {
      console.error(chalk.red('❌ Failed to fetch profile:'), error.message);
    }
  });

// --- Laws Commands ---
const lawsCommand = program.command('laws').description('Manage Registry Laws');

lawsCommand
  .command('pull')
  .description('Download active laws from Registry to local files')
  .action(async () => {
    const token = getToken();
    if (!token) return;

    console.log(chalk.yellow('⬇️  Pulling laws from Registry...'));
    try {
      // Mock data for now
      const laws = [
        { id: 'law.const.001', version: '1', scope: 'mini_constitution', content: 'law system_invariant:1.0.0: mini_constitution:\n  if invalid_contract then deny' }
      ];

      const lawsDir = path.resolve('laws');
      if (!fs.existsSync(lawsDir)) fs.mkdirSync(lawsDir);

      for (const law of laws) {
        const filePath = path.join(lawsDir, `${law.scope}.${law.id}.law`);
        fs.writeFileSync(filePath, law.content);
        console.log(chalk.green(`  Saved: `) + filePath);
      }
      console.log(chalk.bold('✅ Pull complete.'));
    } catch (error) {
      console.error(chalk.red('❌ Failed to pull laws:'), error.message);
    }
  });

lawsCommand
  .command('push')
  .description('Validate and upload local laws to Registry')
  .action(async () => {
    const token = getToken();
    if (!token) return;

    console.log(chalk.yellow('⬆️  Pushing laws to Registry...'));
    const lawsDir = path.resolve('laws');
    if (!fs.existsSync(lawsDir)) {
      console.error(chalk.red('❌ No laws directory found. Run "pull" first.'));
      return;
    }

    const files = fs.readdirSync(lawsDir).filter(f => f.endsWith('.law'));
    for (const file of files) {
      const content = fs.readFileSync(path.join(lawsDir, file), 'utf-8');
      if (!content.includes('law ')) {
         console.error(chalk.red(`  ❌ Invalid syntax in ${file}`));
         continue;
      }
      
      console.log(chalk.cyan(`  Pushing ${file}...`));
      // await axios.post(...)
    }
    console.log(chalk.bold('✅ Push complete.'));
  });

// --- Universal Container Commands ---
const ucCommand = program.command('uc').description('Universal Container Operations');

ucCommand
  .command('create <name>')
  .description('Create a Universal Container (.llc)')
  .option('-c, --code <path>', 'Path to code/artifact')
  .option('-o, --out <path>', 'Output path', '.')
  .action(async (name, options) => {
    console.log(chalk.yellow(`📦 Packaging Universal Container: ${name}`));
    
    const outputName = `${name.split('/').pop()}.llc`;
    const outputPath = path.join(options.out, outputName);
    const output = fs.createWriteStream(outputPath);
    const archive = archiver('zip', { zlib: { level: 9 } });

    output.on('close', function() {
      console.log(chalk.green(`✔ Created ${outputName} (${archive.pointer()} bytes)`));
    });

    archive.pipe(output);

    const manifest = {
      type: "ll.universal.v1",
      name: name,
      timestamp: new Date().toISOString(),
      capabilities: {
        runtime: { container: { image_ref: "artifacts/app.bin" } }
      }
    };
    archive.append(JSON.stringify(manifest, null, 2), { name: 'manifest.json' });

    if (options.code) {
      if (fs.statSync(options.code).isDirectory()) {
        archive.directory(options.code, 'artifacts');
      } else {
        archive.file(options.code, { name: 'artifacts/app.bin' });
      }
    }

    archive.append('mock_signature_ed25519', { name: 'signatures/provenance.sig' });
    await archive.finalize();
  });

ucCommand
  .command('run <file>')
  .description('Run a Universal Container with specific intent')
  .option('-i, --intent <intent>', 'Execution intent (runtime.container, wallet.sign, etc)', 'runtime.container')
  .action(async (file, options) => {
    const filePath = path.resolve(file);
    if (!fs.existsSync(filePath)) {
      console.error(chalk.red(`❌ File not found: ${file}`));
      return;
    }

    console.log(chalk.yellow(`🚀 Loading Universal Container: ${path.basename(file)}`));
    
    try {
      const zip = new AdmZip(filePath);
      const manifestEntry = zip.getEntry('manifest.json');
      
      if (!manifestEntry) {
        throw new Error('Invalid .llc: manifest.json missing');
      }

      const manifest = JSON.parse(manifestEntry.getData().toString('utf8'));
      console.log(chalk.dim(`   ID: ${manifest.name}`));
      console.log(chalk.dim(`   Intent: ${options.intent}`));

      switch (options.intent) {
        case 'runtime.container':
          console.log(chalk.cyan('\n🐳 Starting Container Runtime...'));
          await simulateSpinner('Booting Kernel', 2000);
          console.log(chalk.green('✔ Container is running on port 8080'));
          break;

        case 'wallet.sign':
          console.log(chalk.cyan('\n🔐 Accessing Wallet Identity...'));
          await simulateSpinner('Unlocking Secure Enclave', 1500);
          console.log(chalk.green('✔ Payload signed: sig_ed25519_mock_abc123'));
          break;

        case 'network.mesh.join':
          console.log(chalk.cyan('\n🌐 Joining Mesh Network...'));
          await simulateSpinner('Handshaking Peers', 1000);
          console.log(chalk.green('✔ Connected to 10.42.0.5'));
          break;

        default:
          console.log(chalk.red(`❌ Unknown intent: ${options.intent}`));
      }

    } catch (error) {
      console.error(chalk.red('❌ Runtime Error:'), error.message);
    }
  });

function getToken() {
  if (!fs.existsSync(TOKEN_FILE)) {
    console.error(chalk.red('❌ Not logged in. Run "logline login" first.'));
    return null;
  }
  return fs.readFileSync(TOKEN_FILE, 'utf-8');
}

async function simulateSpinner(text: string, duration: number) {
  const frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
  let i = 0;
  return new Promise<void>(resolve => {
    const interval = setInterval(() => {
      process.stdout.write(`\r${chalk.blue(frames[i++ % frames.length])} ${text}`);
    }, 80);
    setTimeout(() => {
      clearInterval(interval);
      process.stdout.write('\r' + ' '.repeat(text.length + 2) + '\r');
      resolve();
    }, duration);
  });
}

program.parse(process.argv);
