import * as dotenv from 'dotenv';
import { App } from 'octokit';
import * as fs from 'fs';
import * as path from 'path';

dotenv.config({ path: '.env' });
dotenv.config({ path: '../.env.local' });

async function createRepo() {
  console.log('🚀 Initializing GitHub App...');

  const appId = process.env.GITHUB_APP_ID;
  const installationId = process.env.GITHUB_INSTALLATION_ID;
  
  // Try to load private key from file if env var is truncated or file path
  let privateKey = process.env.GITHUB_PRIVATE_KEY;
  
  // Check for pem file in root if env var looks suspicious or user mentioned a file
  const rootPemPath = path.resolve('../minicontratos.2025-11-20.private-key.pem');
  if (fs.existsSync(rootPemPath)) {
    console.log(`🔑 Loading private key from ${rootPemPath}`);
    privateKey = fs.readFileSync(rootPemPath, 'utf-8');
  }

  if (!appId || !privateKey || !installationId) {
    console.error('❌ Missing GitHub App credentials in .env');
    process.exit(1);
  }

  try {
    const app = new App({
      appId,
      privateKey,
    });

    console.log(`🔐 Authenticating as Installation ID: ${installationId}...`);
    const octokit = await app.getInstallationOctokit(Number(installationId));

    const repoName = 'logline-os';
    const orgName = 'powerfarm'; 

    console.log(`📦 Creating repository ${orgName}/${repoName}...`);

    // Check if repo exists
    try {
      await octokit.rest.repos.get({ owner: orgName, repo: repoName });
      console.log(`⚠️ Repository ${orgName}/${repoName} already exists.`);
    } catch (e) {
      if (e.status === 404) {
        // Create repo
        const response = await octokit.rest.repos.createInOrg({
          org: orgName,
          name: repoName,
          description: 'LogLine OS - Universal Container Runtime & AI Governance System',
          private: true, // Default to private for safety
          has_issues: true,
          has_projects: true,
          has_wiki: true,
        });
        console.log(`✅ Repository created: ${response.data.html_url}`);
      } else {
        throw e;
      }
    }

    // Setup remote
    console.log('\n📋 To push your code, run:');
    console.log(`git remote add origin https://x-access-token:${(await octokit.auth({ type: 'installation' }) as any).token}@github.com/${orgName}/${repoName}.git`);
    console.log('git branch -M main');
    console.log('git push -u origin main');

  } catch (error) {
    console.error('❌ Error:', error.message);
    if (error.response) {
      console.error('Response:', error.response.data);
    }
  }
}

createRepo();

