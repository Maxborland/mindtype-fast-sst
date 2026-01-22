# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MindType is a dual-component monorepo:
- **mindtype-site**: Next.js 16 web application for license management and API
- **mindtype-app**: Python desktop application for speech recognition using Whisper

## Commands

### mindtype-site (Web Application)

```bash
cd mindtype-site

# Development
npm install          # Install dependencies
npm run dev          # Development server (port 3000)
npm run build        # Production build
npm run lint         # ESLint

# Testing
npm run test         # Jest unit tests
npm run test:watch   # Watch mode
npm run test:coverage # Coverage report
npm run test:e2e     # Playwright E2E tests
npm run test:e2e:ui  # E2E with interactive UI

# Database
npm run db:migrate   # Run Drizzle migrations
npm run db:studio    # Open Drizzle Studio GUI

# Admin
npm run admin:create # Create admin user interactively
npm run check:config # Verify production config
npm run release:upload # Upload app release
```

### mindtype-app (Desktop Application)

```bash
cd mindtype-app

# Development
python main.py

# Windows build (Nuitka recommended)
.\build\build_windows_nuitka.ps1
.\build\build_all.ps1 -Platform windows

# Linux build
./build/build_linux.sh [version] [--clean]

# macOS build
./build/build_macos.sh [version] [--clean]
```

## Architecture

### mindtype-site

```
src/
├── app/                    # Next.js App Router
│   ├── api/                # RESTful API routes
│   │   ├── admin/          # Admin panel endpoints
│   │   ├── license/        # License validation/deactivation
│   │   ├── releases/       # App release downloads
│   │   ├── crash-report/   # Desktop app crash reporting
│   │   └── payment/        # Robokassa payment webhooks
│   └── [locale]/           # Localized pages (en, ru, es, de, fr, zh)
├── components/             # React components
├── db/                     # Drizzle ORM
│   ├── index.ts            # Database initialization
│   └── schema.ts           # SQLite schema (licenses, activations, orders, etc.)
├── lib/                    # Utilities
│   ├── auth.ts             # NextAuth configuration
│   ├── license.ts          # License key generation
│   ├── rate-limit.ts       # Rate limiting
│   └── robokassa.ts        # Payment integration
├── i18n/                   # next-intl configuration
└── middleware.ts           # i18n routing middleware
```

**Key patterns:**
- API routes use Zod for request validation
- JWT-based authentication via NextAuth
- SQLite database with Drizzle ORM
- Rate limiting on API endpoints
- next-intl for 6-language support

### mindtype-app

```
app/
├── main.py                 # Entry point and GUI loop
├── transcriber_onnx.py     # Whisper ONNX speech recognition
├── assistant.py            # AI assistant integration
├── licensing/              # License validation
├── platform/               # Platform-specific code (Windows, macOS, Linux)
├── llm/                    # LLM provider integrations (Ollama, OpenRouter)
├── ui/                     # PyQt6 UI components
├── crash_reporter.py       # Error reporting to backend
├── updater.py              # Auto-update mechanism
└── env.py                  # Configuration (APP_VERSION lives here)
```

**Key patterns:**
- PyQt6 for cross-platform UI
- Whisper ONNX for speech-to-text
- License validation against mindtype-site API
- Platform abstraction in `platform/` directory

## Database Schema (mindtype-site)

Core tables in `src/db/schema.ts`:
- `licenses`: License keys with plan type (personal/pro/team) and expiration
- `activations`: Device activations per license (limited per plan)
- `orders`: Payment orders from Robokassa
- `admin_users`: Control panel administrators
- `crash_reports`: Desktop app crash reports
- `releases`: App release metadata with SHA256 checksums

## Environment Setup (mindtype-site)

Copy `.env.example` to `.env.local`:
- `DATABASE_URL`: SQLite path
- `ADMIN_JWT_SECRET`: JWT secret for admin auth
- `ROBOKASSA_*`: Payment provider credentials
- `SMTP_*`: Email sending configuration
- `LICENSE_KEY_SECRET`: License key generation secret

## Deployment

mindtype-site uses Docker with Caddy reverse proxy:
- `docker-compose.yml` for orchestration
- GitHub Actions deploys to VPS on push to main
- Health check at `/api/health`
