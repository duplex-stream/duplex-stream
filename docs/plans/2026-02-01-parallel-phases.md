# Parallel Phases Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete Phases 3-5 in parallel: Onboarding API, Dashboard API integration, and Desktop Auth flow.

**Architecture:** Three independent work streams that can run concurrently:
- Stream A: Web onboarding → API workspace creation
- Stream B: Web dashboard → API decisions integration
- Stream C: Desktop auth → Web callback → deep-link token exchange

**Tech Stack:** TanStack Start, Hono API, Tauri (Rust), WorkOS AuthKit

---

## Stream Overview

| Stream | Owner | Focus | Files |
|--------|-------|-------|-------|
| A | Agent 1 | Onboarding + Workspace API | `apps/api/`, `apps/web/.../onboarding/` |
| B | Agent 2 | Dashboard API Integration | `apps/web/.../dashboard.tsx`, `apps/web/src/lib/api.ts` |
| C | Agent 3 | Desktop Auth Flow | `apps/desktop/src-tauri/`, `apps/web/.../auth/callback.tsx` |

---

## Stream A: Onboarding + Workspace API

### Task A1: Add workspace API route

**Files:**
- Create: `apps/api/src/routes/workspaces.ts`
- Modify: `apps/api/src/index.ts`

**Step 1: Create workspace route**

```typescript
// apps/api/src/routes/workspaces.ts
import { Hono } from 'hono'
import { z } from 'zod/v4'
import { zValidator } from '@hono/zod-validator'
import type { HonoEnv } from '../context'

const workspaces = new Hono<HonoEnv>()

const createWorkspaceSchema = z.object({
  name: z.string().min(1).max(100),
})

workspaces.post('/', zValidator('json', createWorkspaceSchema), async (c) => {
  const { name } = c.req.valid('json')
  const userId = c.get('userId')

  // For now, just return success - workspace is implicit per user
  return c.json({
    id: userId,
    name,
    createdAt: new Date().toISOString(),
  })
})

workspaces.get('/current', async (c) => {
  const userId = c.get('userId')
  // Return user's default workspace
  return c.json({
    id: userId,
    name: 'My Workspace',
  })
})

export default workspaces
```

**Step 2: Register route in index.ts**

Add to `apps/api/src/index.ts`:
```typescript
import workspacesRoutes from './routes/workspaces'

// After other route registrations
app.use('/workspaces/*', authMiddleware())
app.route('/workspaces', workspacesRoutes)
```

**Step 3: Commit**
```bash
git add apps/api/src/routes/workspaces.ts apps/api/src/index.ts
git commit -m "feat(api): add workspace API route"
```

### Task A2: Connect onboarding to API

**Files:**
- Modify: `apps/web/src/routes/onboarding/index.tsx`
- Modify: `apps/web/src/lib/api.ts`

**Step 1: Add createWorkspace to API client**

Add to `apps/web/src/lib/api.ts`:
```typescript
async createWorkspace(name: string): Promise<{ id: string; name: string }> {
  return this.fetch('/workspaces', {
    method: 'POST',
    body: JSON.stringify({ name }),
  })
}
```

**Step 2: Update onboarding to call API**

Replace the handleSubmit in `apps/web/src/routes/onboarding/index.tsx`:
```typescript
import { createServerFn } from '@tanstack/react-start'
import { getAuth } from '@workos/authkit-tanstack-react-start'
import { createApiClient } from '~/lib/api'

const createWorkspace = createServerFn({ method: 'POST' })
  .inputValidator((data: { name: string }) => data)
  .handler(async ({ data }) => {
    const auth = await getAuth()
    if (!auth.accessToken) throw new Error('Not authenticated')

    const api = createApiClient(auth.accessToken)
    return api.createWorkspace(data.name)
  })

// In component:
const handleSubmit = async (e: React.FormEvent) => {
  e.preventDefault()
  if (!workspaceName.trim()) return

  setIsSubmitting(true)
  try {
    await createWorkspace({ data: { name: workspaceName } })
    navigate({ to: '/dashboard' })
  } catch (error) {
    console.error('Failed to create workspace:', error)
    setIsSubmitting(false)
  }
}
```

**Step 3: Commit**
```bash
git add apps/web/src/routes/onboarding/index.tsx apps/web/src/lib/api.ts
git commit -m "feat(web): connect onboarding to workspace API"
```

---

## Stream B: Dashboard API Integration

### Task B1: Replace mock data with API calls

**Files:**
- Modify: `apps/web/src/routes/_authed/dashboard.tsx`

**Step 1: Update fetchDecisions to use real API**

Replace the mock `fetchDecisions` in dashboard.tsx:
```typescript
import { getAuth } from '@workos/authkit-tanstack-react-start'
import { createApiClient } from '~/lib/api'

const fetchDecisions = createServerFn({ method: 'GET' })
  .inputValidator(
    (data: { page?: number; pageSize?: number; search?: string }) => data,
  )
  .handler(async ({ data }) => {
    const auth = await getAuth()
    if (!auth.accessToken) {
      return { decisions: [], total: 0, page: 1, pageSize: 10 }
    }

    const api = createApiClient(auth.accessToken)
    const response = await api.getDecisions({
      page: data.page,
      pageSize: data.pageSize,
      search: data.search,
    })

    return {
      decisions: response.data,
      total: response.total,
      page: response.page,
      pageSize: response.pageSize,
    }
  })
```

**Step 2: Commit**
```bash
git add apps/web/src/routes/_authed/dashboard.tsx
git commit -m "feat(web): connect dashboard to decisions API"
```

### Task B2: Update decision detail page

**Files:**
- Modify: `apps/web/src/routes/_authed/decisions.$id.tsx`

**Step 1: Fetch real decision by ID**

Update the loader to use real API:
```typescript
import { getAuth } from '@workos/authkit-tanstack-react-start'
import { createApiClient } from '~/lib/api'

const fetchDecision = createServerFn({ method: 'GET' })
  .inputValidator((data: { id: string }) => data)
  .handler(async ({ data }) => {
    const auth = await getAuth()
    if (!auth.accessToken) throw new Error('Not authenticated')

    const api = createApiClient(auth.accessToken)
    return api.getDecision(data.id)
  })

export const Route = createFileRoute('/_authed/decisions/$id')({
  loader: async ({ params }) => {
    return fetchDecision({ data: { id: params.id } })
  },
  component: DecisionDetailPage,
})
```

**Step 2: Commit**
```bash
git add apps/web/src/routes/_authed/decisions.\$id.tsx
git commit -m "feat(web): connect decision detail to API"
```

---

## Stream C: Desktop Auth Flow

### Task C1: Fix callback to return access token

**Files:**
- Modify: `apps/web/src/routes/auth/callback.tsx`

**Step 1: Update callback to pass token for desktop flow**

The current callback redirects with `user_id`, but desktop needs the access token.
Update callback to use a secure token exchange:

```typescript
import { createFileRoute } from '@tanstack/react-router'
import { handleCallbackRoute, getAuth } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/callback')({
  server: {
    handlers: {
      GET: handleCallbackRoute({
        returnPathname: '/dashboard',
        onSuccess: async ({ user }) => {
          console.log('User authenticated:', user.email)
        },
        onError: ({ error }) => {
          console.error('Authentication failed:', error)
          return new Response(null, {
            status: 302,
            headers: { Location: '/auth/login' },
          })
        },
      }),
    },
  },
  validateSearch: (search: Record<string, unknown>) => ({
    desktop: search.desktop as string | undefined,
  }),
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps }) => {
    // After successful auth, check if desktop flow
    if (deps.search.desktop === 'true') {
      const auth = await getAuth()
      if (auth.accessToken) {
        // Redirect to desktop app with token
        const callbackUrl = `duplex://auth/callback?token=${encodeURIComponent(auth.accessToken)}`
        throw redirect({ href: callbackUrl })
      }
    }
    throw redirect({ to: '/dashboard' })
  },
  component: () => null,
})
```

**Step 2: Commit**
```bash
git add apps/web/src/routes/auth/callback.tsx
git commit -m "feat(web): pass access token in desktop auth callback"
```

### Task C2: Update desktop to refresh menu after auth

**Files:**
- Modify: `apps/desktop/src-tauri/src/main.rs`

**Step 1: Add menu refresh after storing token**

In the deep-link handler, after storing the token, emit an event to refresh the menu:

```rust
// In the deep-link handler, after successful token storage:
if let Err(e) = store_token_in_keyring(&token) {
    tracing::error!("Failed to store token in keyring: {}", e);
} else {
    tracing::info!("Token stored successfully");
    // Emit event to trigger menu refresh
    let _ = app_handle.emit("auth-state-changed", true);
}
```

**Step 2: Commit**
```bash
git add apps/desktop/src-tauri/src/main.rs
git commit -m "feat(desktop): refresh menu after auth state change"
```

### Task C3: Update desktop state param handling

**Files:**
- Modify: `apps/web/src/routes/auth/desktop.tsx`

**Step 1: Ensure desktop route passes state correctly**

The desktop route should pass `desktop=true` in the OAuth state:

```typescript
import { createFileRoute, redirect } from '@tanstack/react-router'
import { getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/desktop')({
  loader: async () => {
    // Pass desktop=true in state so callback knows to redirect to app
    const signInUrl = await getSignInUrl({
      state: JSON.stringify({ desktop: true }),
    })
    throw redirect({ href: signInUrl })
  },
  component: () => null,
})
```

**Step 2: Commit**
```bash
git add apps/web/src/routes/auth/desktop.tsx
git commit -m "feat(web): encode desktop flag in OAuth state"
```

---

## Execution Commands

### Run all three streams in parallel:

```bash
# Terminal 1 - Stream A (Onboarding)
cd /path/to/duplex-stream
# Work on Tasks A1, A2

# Terminal 2 - Stream B (Dashboard)
cd /path/to/duplex-stream
# Work on Tasks B1, B2

# Terminal 3 - Stream C (Desktop Auth)
cd /path/to/duplex-stream
# Work on Tasks C1, C2, C3
```

### Test commands:

```bash
# Test API
curl -X POST http://localhost:8787/workspaces \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Workspace"}'

# Test web app
pnpm dev  # from apps/web

# Test desktop
cargo tauri dev  # from apps/desktop
```

---

## Merge Strategy

1. Each stream commits to a feature branch
2. All streams merge to `main` when complete
3. Run integration test of full flow:
   - Desktop "Sign In" → Browser → WorkOS → Callback → Desktop deep-link → Token stored
   - Web login → Dashboard shows real decisions
   - Onboarding creates workspace
