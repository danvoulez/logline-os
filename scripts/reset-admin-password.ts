import { DataSource } from 'typeorm';
import * as bcrypt from 'bcrypt';
import * as dotenv from 'dotenv';

dotenv.config({ path: '.env' });
dotenv.config({ path: '../.env.local' });

const dataSource = new DataSource({
  type: 'postgres',
  url: process.env.POSTGRES_URL || process.env.DATABASE_URL,
  ssl: process.env.NODE_ENV === 'production' ? { rejectUnauthorized: false } : undefined,
});

async function reset() {
  console.log('🔄 Connecting to database...');
  try {
    await dataSource.initialize();
  } catch (e) {
    console.log('Trying local connection fallback...');
    // Fallback for local if env vars are messy
    await dataSource.setOptions({
      url: 'postgres://neondb_owner:npg_QX0BWHcAyno6@ep-shiny-thunder-ahpd4na0-pooler.c-3.us-east-1.aws.neon.tech/neondb?sslmode=require',
      ssl: { rejectUnauthorized: false }
    });
    await dataSource.initialize();
  }

  const email = 'dan@danvoulez.com';
  const newPassword = 'logline-founder'; // SENHA PADRÃO
  const hash = await bcrypt.hash(newPassword, 10);

  console.log(`🔑 Resetting password for ${email}...`);
  
  await dataSource.query(
    `UPDATE users SET password_hash = '${hash}' WHERE email = '${email}'`
  );

  console.log(`✅ Password reset successfully.`);
  console.log(`📧 User: ${email}`);
  console.log(`Pwrd: ${newPassword}`);
  
  await dataSource.destroy();
}

reset().catch(err => {
  console.error('❌ Error:', err);
  process.exit(1);
});

