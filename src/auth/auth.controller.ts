import {
  Controller,
  Post,
  Get,
  Patch,
  Body,
  UseGuards,
  HttpCode,
  HttpStatus,
  BadRequestException,
  Req,
  Param,
} from '@nestjs/common';
import { AuthService } from './auth.service';
import { JwtAuthGuard } from './guards/jwt-auth.guard';
import { CurrentUser } from './decorators/current-user.decorator';
import { RegisterDto } from './dto/register.dto';
import { LoginDto } from './dto/login.dto';
import { RefreshTokenDto } from './dto/refresh-token.dto';
import { CreateApiKeyDto } from './dto/create-api-key.dto';
import type { JwtPayload } from './auth.service';
import * as crypto from 'crypto';

// In-memory store for CLI auth sessions (Use Redis in production)
// TODO: Move to Redis or DB for persistence across restarts
const cliSessions = new Map<string, { status: 'pending' | 'approved'; userId?: string; expires: number }>();

@Controller('auth')
export class AuthController {
  constructor(private authService: AuthService) {}

  // --- Remote Auth (CLI Login) ---

  @Post('cli/session')
  async createCliSession() {
    const sessionId = crypto.randomUUID();
    // 5 minutes TTL
    cliSessions.set(sessionId, { 
      status: 'pending', 
      expires: Date.now() + 5 * 60 * 1000 
    });
    
    const frontendUrl = process.env.FRONTEND_URL || 'http://localhost:3000';
    
    return {
      session_id: sessionId,
      url: `${frontendUrl}/auth/cli?code=${sessionId}`,
      qr_code_data: `${frontendUrl}/auth/cli?code=${sessionId}`
    };
  }

  @Get('cli/session/:id')
  async pollCliSession(@Param('id') id: string) {
    const session = cliSessions.get(id);
    
    if (!session) throw new BadRequestException('Session not found or expired');
    if (Date.now() > session.expires) {
      cliSessions.delete(id);
      throw new BadRequestException('Session expired');
    }

    if (session.status === 'approved' && session.userId) {
      // Generate token for the user
      const user = await this.authService.getUserById(session.userId);
      const tokens = await this.authService.generateTokens(user); // Fixed: using public method
      
      cliSessions.delete(id); // Consume session immediately (OTP style)
      return { status: 'approved', ...tokens };
    }

    return { status: 'pending' };
  }

  @Post('cli/approve')
  @UseGuards(JwtAuthGuard)
  async approveCliSession(@Body() body: { code: string }, @CurrentUser() user: JwtPayload) {
    const { code } = body;
    const session = cliSessions.get(code);
    
    if (!session) throw new BadRequestException('Session not found or expired');
    
    session.status = 'approved';
    session.userId = user.sub;
    cliSessions.set(code, session); // Update map
    
    return { message: 'Session approved' };
  }

  // --- Existing Endpoints ---

  @Post('register')
  @HttpCode(HttpStatus.CREATED)
  async register(@Body() registerDto: RegisterDto) {
    const { user, tokens } = await this.authService.register(
      registerDto.email,
      registerDto.password,
      registerDto.name,
      registerDto.cpf,
      registerDto.tenant_id,
    );

    // Don't return password_hash
    const { password_hash, ...userWithoutPassword } = user;

    return {
      user: userWithoutPassword,
      ...tokens,
    };
  }

  @Post('login')
  @HttpCode(HttpStatus.OK)
  async login(@Body() loginDto: LoginDto, @Req() req?: any) {
    const ipAddress = req?.ip || (Array.isArray(req?.headers?.['x-forwarded-for']) 
      ? req?.headers['x-forwarded-for'][0] 
      : req?.headers?.['x-forwarded-for']) || undefined;
    const userAgent = req?.headers?.['user-agent'] || undefined;

    const { user, tokens } = await this.authService.login(
      loginDto.email,
      loginDto.password,
      ipAddress as string,
      userAgent,
    );

    // Don't return password_hash
    const { password_hash, ...userWithoutPassword } = user;

    return {
      user: userWithoutPassword,
      ...tokens,
    };
  }

  @Post('refresh')
  @HttpCode(HttpStatus.OK)
  async refresh(@Body() refreshTokenDto: RefreshTokenDto) {
    return this.authService.refreshToken(refreshTokenDto.refresh_token);
  }

  @Post('logout')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async logout(@Body() refreshTokenDto: RefreshTokenDto, @CurrentUser() user: JwtPayload) {
    await this.authService.logout(refreshTokenDto.refresh_token, user.sub);
    return { message: 'Logged out successfully' };
  }

  @Get('me')
  @UseGuards(JwtAuthGuard)
  async getCurrentUser(@CurrentUser() user: JwtPayload) {
    const fullUser = await this.authService.getUserById(user.sub);

    if (!fullUser) {
      throw new BadRequestException('User not found');
    }

    const { password_hash, ...userWithoutPassword } = fullUser;

    return userWithoutPassword;
  }

  @Patch('me')
  @UseGuards(JwtAuthGuard)
  async updateCurrentUser(
    @CurrentUser() user: JwtPayload,
    @Body() updateData: { name?: string; avatar_url?: string },
  ) {
    // This would typically update the user in the database
    // For now, it's a placeholder
    return { message: 'User update not yet implemented' };
  }

  @Post('api-keys')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.CREATED)
  async createApiKey(
    @CurrentUser() user: JwtPayload,
    @Body() createApiKeyDto: CreateApiKeyDto,
  ) {
    const expiresAt = createApiKeyDto.expires_at
      ? new Date(createApiKeyDto.expires_at)
      : undefined;

    const { apiKey, key } = await this.authService.createApiKey(
      user.sub,
      createApiKeyDto.name,
      createApiKeyDto.permissions || [],
      expiresAt,
    );

    // Return API key only once (for security)
    return {
      id: apiKey.id,
      name: apiKey.name,
      key, // Only returned on creation
      permissions: apiKey.permissions,
      expires_at: apiKey.expires_at,
      created_at: apiKey.created_at,
    };
  }

  @Get('api-keys')
  @UseGuards(JwtAuthGuard)
  async listApiKeys(@CurrentUser() user: JwtPayload) {
    // This would typically list all API keys for the user
    // For now, it's a placeholder
    return { message: 'API key listing not yet implemented' };
  }

  @Post('api-keys/:id/revoke')
  @UseGuards(JwtAuthGuard)
  @HttpCode(HttpStatus.OK)
  async revokeApiKey(@CurrentUser() user: JwtPayload, @Param('id') id: string) {
    await this.authService.revokeApiKey(id, user.sub);
    return { message: 'API key revoked successfully' };
  }
}

