# Extraction Pipeline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an extraction pipeline that parses Claude Code conversations and extracts structured decisions with reasoning chains and dependencies.

**Architecture:** Two-phase LLM extraction (identify decisions → extract reasoning), Cloudflare Workflows for durability, D1 for storage, Drizzle ORM for type-safe queries.

**Tech Stack:** Drizzle ORM, D1, Cloudflare Workflows, Vercel AI SDK, Claude Sonnet 4.5, Hono, Zod

---

## Task 1: Create packages/db Package Structure

**Files:**
- Create: `packages/db/package.json`
- Create: `packages/db/tsconfig.json`
- Create: `packages/db/src/index.ts`

**Step 1: Create package.json**

```json
{
  "name": "@repo/db",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": "./src/index.ts",
    "./schema": "./src/schema/index.ts",
    "./types": "./src/types.ts"
  },
  "scripts": {
    "generate": "drizzle-kit generate",
    "migrate": "drizzle-kit migrate",
    "studio": "drizzle-kit studio"
  },
  "dependencies": {
    "drizzle-orm": "^0.38.0"
  },
  "devDependencies": {
    "@repo/typescript-config": "workspace:*",
    "drizzle-kit": "^0.30.0",
    "typescript": "catalog:"
  }
}
```

**Step 2: Create tsconfig.json**

```json
{
  "extends": "@repo/typescript-config/lib.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"]
}
```

**Step 3: Create src/index.ts**

```typescript
export * from './schema/index.js'
export * from './types.js'
```

**Step 4: Install dependencies**

Run: `pnpm install`

**Step 5: Commit**

```bash
git add packages/db
git commit -m "feat(db): scaffold packages/db with Drizzle setup"
```

---

## Task 2: Create Drizzle Schema

**Files:**
- Create: `packages/db/src/schema/extraction.ts`
- Create: `packages/db/src/schema/index.ts`

**Step 1: Create extraction schema**

Create `packages/db/src/schema/extraction.ts`:

```typescript
import { sqliteTable, text, integer, real } from 'drizzle-orm/sqlite-core'
import { relations } from 'drizzle-orm'

// Conversations table
export const conversations = sqliteTable('conversations', {
  id: text('id').primaryKey(),
  orgId: text('org_id').notNull(),
  workspaceId: text('workspace_id').notNull(),
  source: text('source', {
    enum: ['claude-code', 'claude-web', 'cursor', 'other'],
  }).notNull(),
  sourcePath: text('source_path').notNull(),
  projectPath: text('project_path').notNull(),
  sessionId: text('session_id'),
  messageCount: integer('message_count').notNull(),
  createdAt: text('created_at').notNull(),
  extractedAt: text('extracted_at'),
})

// Messages table
export const messages = sqliteTable('messages', {
  id: text('id').primaryKey(),
  conversationId: text('conversation_id')
    .notNull()
    .references(() => conversations.id, { onDelete: 'cascade' }),
  index: integer('index').notNull(),
  role: text('role', { enum: ['user', 'assistant', 'system'] }).notNull(),
  content: text('content').notNull(),
  thinking: text('thinking'),
  timestamp: text('timestamp'),
})

// Decisions table
export const decisions = sqliteTable('decisions', {
  id: text('id').primaryKey(),
  conversationId: text('conversation_id')
    .notNull()
    .references(() => conversations.id, { onDelete: 'cascade' }),
  orgId: text('org_id').notNull(),
  workspaceId: text('workspace_id').notNull(),
  title: text('title').notNull(),
  summary: text('summary').notNull(),
  reasoning: text('reasoning').notNull(),
  status: text('status', {
    enum: ['active', 'superseded', 'tentative'],
  }).notNull(),
  confidence: real('confidence').notNull(),
  extractedAt: text('extracted_at').notNull(),
})

// Decision appearances (non-contiguous locations)
export const decisionAppearances = sqliteTable('decision_appearances', {
  id: text('id').primaryKey(),
  decisionId: text('decision_id')
    .notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  messageStart: integer('message_start').notNull(),
  messageEnd: integer('message_end').notNull(),
  type: text('type', {
    enum: ['introduced', 'elaborated', 'modified', 'reaffirmed'],
  }).notNull(),
  context: text('context'),
})

// Alternatives considered
export const alternatives = sqliteTable('alternatives', {
  id: text('id').primaryKey(),
  decisionId: text('decision_id')
    .notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  description: text('description').notNull(),
  whyRejected: text('why_rejected').notNull(),
})

// Decision dependencies (graph edges)
export const decisionDependencies = sqliteTable('decision_dependencies', {
  id: text('id').primaryKey(),
  fromDecisionId: text('from_decision_id')
    .notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  toDecisionRef: text('to_decision_ref').notNull(),
})

// Relations
export const conversationsRelations = relations(conversations, ({ many }) => ({
  messages: many(messages),
  decisions: many(decisions),
}))

export const messagesRelations = relations(messages, ({ one }) => ({
  conversation: one(conversations, {
    fields: [messages.conversationId],
    references: [conversations.id],
  }),
}))

export const decisionsRelations = relations(decisions, ({ one, many }) => ({
  conversation: one(conversations, {
    fields: [decisions.conversationId],
    references: [conversations.id],
  }),
  appearances: many(decisionAppearances),
  alternatives: many(alternatives),
  dependencies: many(decisionDependencies),
}))

export const decisionAppearancesRelations = relations(
  decisionAppearances,
  ({ one }) => ({
    decision: one(decisions, {
      fields: [decisionAppearances.decisionId],
      references: [decisions.id],
    }),
  })
)

export const alternativesRelations = relations(alternatives, ({ one }) => ({
  decision: one(decisions, {
    fields: [alternatives.decisionId],
    references: [decisions.id],
  }),
}))

export const decisionDependenciesRelations = relations(
  decisionDependencies,
  ({ one }) => ({
    fromDecision: one(decisions, {
      fields: [decisionDependencies.fromDecisionId],
      references: [decisions.id],
    }),
  })
)
```

**Step 2: Create schema index**

Create `packages/db/src/schema/index.ts`:

```typescript
export * from './extraction.js'
```

**Step 3: Commit**

```bash
git add packages/db/src/schema
git commit -m "feat(db): add Drizzle schema for extraction tables"
```

---

## Task 3: Create Type Derivations

**Files:**
- Create: `packages/db/src/types.ts`

**Step 1: Create types file**

```typescript
import type { InferSelectModel, InferInsertModel } from 'drizzle-orm'
import * as schema from './schema/index.js'

// Base types from schema
export type Conversation = InferSelectModel<typeof schema.conversations>
export type NewConversation = InferInsertModel<typeof schema.conversations>

export type Message = InferSelectModel<typeof schema.messages>
export type NewMessage = InferInsertModel<typeof schema.messages>

export type Decision = InferSelectModel<typeof schema.decisions>
export type NewDecision = InferInsertModel<typeof schema.decisions>

export type DecisionAppearance = InferSelectModel<typeof schema.decisionAppearances>
export type NewDecisionAppearance = InferInsertModel<typeof schema.decisionAppearances>

export type Alternative = InferSelectModel<typeof schema.alternatives>
export type NewAlternative = InferInsertModel<typeof schema.alternatives>

export type DecisionDependency = InferSelectModel<typeof schema.decisionDependencies>
export type NewDecisionDependency = InferInsertModel<typeof schema.decisionDependencies>

// Composite types for queries
export type DecisionWithRelations = Decision & {
  appearances: DecisionAppearance[]
  alternatives: Alternative[]
  dependencies: DecisionDependency[]
  conversation?: Conversation
}

export type ConversationWithRelations = Conversation & {
  messages: Message[]
  decisions: Decision[]
}
```

**Step 2: Commit**

```bash
git add packages/db/src/types.ts
git commit -m "feat(db): add type derivations from Drizzle schema"
```

---

## Task 4: Create Drizzle Config and Migration

**Files:**
- Create: `packages/db/drizzle.config.ts`

**Step 1: Create drizzle config**

```typescript
import { defineConfig } from 'drizzle-kit'

export default defineConfig({
  schema: './src/schema/index.ts',
  out: './drizzle',
  dialect: 'sqlite',
})
```

**Step 2: Generate migration**

Run: `cd packages/db && pnpm generate`

Expected: Creates `packages/db/drizzle/` with migration files

**Step 3: Commit**

```bash
git add packages/db/drizzle.config.ts packages/db/drizzle
git commit -m "feat(db): add Drizzle config and initial migration"
```

---

## Task 5: Create packages/extraction Package Structure

**Files:**
- Create: `packages/extraction/package.json`
- Create: `packages/extraction/tsconfig.json`
- Create: `packages/extraction/src/index.ts`

**Step 1: Create package.json**

```json
{
  "name": "@repo/extraction",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": "./src/index.ts",
    "./parsers": "./src/parsers/index.ts",
    "./prompts": "./src/prompts/index.ts"
  },
  "dependencies": {
    "@ai-sdk/anthropic": "^1.0.0",
    "ai": "^4.0.0",
    "zod": "catalog:"
  },
  "devDependencies": {
    "@repo/typescript-config": "workspace:*",
    "typescript": "catalog:"
  }
}
```

**Step 2: Create tsconfig.json**

```json
{
  "extends": "@repo/typescript-config/lib.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"]
}
```

**Step 3: Create src/index.ts**

```typescript
export * from './parsers/index.js'
export * from './prompts/index.js'
export * from './transcript.js'
export * from './schemas.js'
```

**Step 4: Install dependencies**

Run: `pnpm install`

**Step 5: Commit**

```bash
git add packages/extraction
git commit -m "feat(extraction): scaffold packages/extraction"
```

---

## Task 6: Create Claude Code Parser

**Files:**
- Create: `packages/extraction/src/parsers/claude-code.ts`
- Create: `packages/extraction/src/parsers/index.ts`

**Step 1: Create parser types and implementation**

Create `packages/extraction/src/parsers/claude-code.ts`:

```typescript
interface ClaudeCodeEvent {
  type: string
  message?: {
    role: string
    content: string | Array<{ type: string; text?: string; thinking?: string }>
  }
  timestamp?: string
  sessionId?: string
}

export interface ParsedMessage {
  index: number
  role: 'user' | 'assistant' | 'system'
  content: string
  thinking?: string
  timestamp?: Date
}

export interface ParsedConversation {
  sessionId: string
  projectPath: string
  messages: ParsedMessage[]
  createdAt: Date
}

export interface ParseOptions {
  sourcePath?: string
}

export function parseClaudeCodeSession(
  content: string,
  options: ParseOptions = {}
): ParsedConversation {
  const lines = content.trim().split('\n')

  const messages: ParsedMessage[] = []
  let sessionId = ''
  let projectPath = ''
  let earliestTimestamp: Date | null = null

  for (const line of lines) {
    try {
      const event: ClaudeCodeEvent = JSON.parse(line)

      if (event.sessionId && !sessionId) {
        sessionId = event.sessionId
      }

      if (event.timestamp) {
        const ts = new Date(event.timestamp)
        if (!earliestTimestamp || ts < earliestTimestamp) {
          earliestTimestamp = ts
        }
      }

      if (event.type === 'user' && event.message) {
        messages.push({
          index: messages.length,
          role: 'user',
          content:
            typeof event.message.content === 'string'
              ? event.message.content
              : '',
          timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
        })
      }

      if (event.type === 'assistant' && event.message) {
        const contentArray = Array.isArray(event.message.content)
          ? event.message.content
          : []

        const textContent = contentArray
          .filter((block) => block.type === 'text')
          .map((block) => block.text || '')
          .join('\n')

        const thinkingContent = contentArray
          .filter((block) => block.type === 'thinking')
          .map((block) => block.thinking || '')
          .join('\n')

        if (textContent || thinkingContent) {
          messages.push({
            index: messages.length,
            role: 'assistant',
            content: textContent,
            thinking: thinkingContent || undefined,
            timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
          })
        }
      }

      if (event.type === 'system' && event.message) {
        messages.push({
          index: messages.length,
          role: 'system',
          content:
            typeof event.message.content === 'string'
              ? event.message.content
              : JSON.stringify(event.message.content),
          timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
        })
      }
    } catch {
      continue
    }
  }

  if (options.sourcePath) {
    const match = options.sourcePath.match(/projects\/([^/]+)\//)
    if (match) {
      projectPath = match[1].replace(/-/g, '/')
      if (projectPath.startsWith('/')) {
        projectPath = projectPath.slice(1)
      }
    }
  }

  return {
    sessionId,
    projectPath,
    messages,
    createdAt: earliestTimestamp || new Date(),
  }
}
```

**Step 2: Create parsers index**

Create `packages/extraction/src/parsers/index.ts`:

```typescript
export * from './claude-code.js'
```

**Step 3: Commit**

```bash
git add packages/extraction/src/parsers
git commit -m "feat(extraction): add Claude Code session parser"
```

---

## Task 7: Create Transcript Builder

**Files:**
- Create: `packages/extraction/src/transcript.ts`

**Step 1: Create transcript utilities**

```typescript
import type { ParsedMessage } from './parsers/claude-code.js'

export function buildTranscript(messages: ParsedMessage[]): string {
  return messages
    .map((m) => {
      let text = `[${m.index}] ${m.role.toUpperCase()}: ${m.content}`
      if (m.thinking) {
        text += `\n[${m.index}] THINKING: ${m.thinking}`
      }
      return text
    })
    .join('\n\n')
}

interface Appearance {
  messageStart: number
  messageEnd: number
  type: string
}

interface ContextWindowOptions {
  buffer?: number
}

export function buildContextWindow(
  messages: ParsedMessage[],
  appearances: Appearance[],
  options: ContextWindowOptions = {}
): string {
  const { buffer = 2 } = options

  const indices = new Set<number>()

  for (const appearance of appearances) {
    const start = Math.max(0, appearance.messageStart - buffer)
    const end = Math.min(messages.length - 1, appearance.messageEnd + buffer)

    for (let i = start; i <= end; i++) {
      indices.add(i)
    }
  }

  const sortedIndices = Array.from(indices).sort((a, b) => a - b)

  return sortedIndices
    .map((i) => {
      const m = messages[i]
      let text = `[${m.index}] ${m.role.toUpperCase()}: ${m.content}`
      if (m.thinking) {
        text += `\n[${m.index}] THINKING: ${m.thinking}`
      }
      return text
    })
    .join('\n\n')
}
```

**Step 2: Commit**

```bash
git add packages/extraction/src/transcript.ts
git commit -m "feat(extraction): add transcript building utilities"
```

---

## Task 8: Create Zod Schemas for LLM Responses

**Files:**
- Create: `packages/extraction/src/schemas.ts`

**Step 1: Create LLM response schemas**

```typescript
import { z } from 'zod'

// Phase 1: Decision identification
export const DecisionAppearanceSchema = z.object({
  messageStart: z.number(),
  messageEnd: z.number(),
  type: z.enum(['introduced', 'elaborated', 'modified', 'reaffirmed']),
})

export const DecisionCandidateSchema = z.object({
  tempId: z.string(),
  title: z.string(),
  appearances: z.array(DecisionAppearanceSchema),
  confidence: z.number().min(0).max(1),
})

export const IdentificationResponseSchema = z.object({
  decisions: z.array(DecisionCandidateSchema),
})

// Phase 2: Decision extraction
export const AlternativeSchema = z.object({
  description: z.string(),
  whyRejected: z.string(),
})

export const ExtractedDecisionSchema = z.object({
  summary: z.string(),
  reasoning: z.string(),
  alternativesConsidered: z.array(AlternativeSchema),
  status: z.enum(['active', 'superseded', 'tentative']),
  dependsOn: z.array(z.string()),
  confidence: z.number().min(0).max(1),
})

// Inferred types
export type DecisionAppearance = z.infer<typeof DecisionAppearanceSchema>
export type DecisionCandidate = z.infer<typeof DecisionCandidateSchema>
export type IdentificationResponse = z.infer<typeof IdentificationResponseSchema>
export type Alternative = z.infer<typeof AlternativeSchema>
export type ExtractedDecision = z.infer<typeof ExtractedDecisionSchema>
```

**Step 2: Commit**

```bash
git add packages/extraction/src/schemas.ts
git commit -m "feat(extraction): add Zod schemas for LLM responses"
```

---

## Task 9: Create Prompt Templates

**Files:**
- Create: `packages/extraction/src/prompts/identification.ts`
- Create: `packages/extraction/src/prompts/extraction.ts`
- Create: `packages/extraction/src/prompts/index.ts`

**Step 1: Create identification prompt**

Create `packages/extraction/src/prompts/identification.ts`:

```typescript
export function buildIdentificationPrompt(transcript: string): string {
  return `You are analyzing a conversation to identify decisions that were made.

A decision is:
- An explicit choice between alternatives
- A commitment to an approach, architecture, or implementation
- A constraint or principle that guides other choices

For each decision, provide:
- tempId: Temporary identifier (e.g., "decision_1", "decision_2")
- title: Short descriptive name (5-10 words)
- appearances: Where this decision appears in the conversation
  - messageStart: First message index of this appearance
  - messageEnd: Last message index of this appearance
  - type: One of "introduced", "elaborated", "modified", "reaffirmed"
- confidence: 0-1 how certain you are this is a real decision (not just discussion)

IMPORTANT: Decisions often evolve across a conversation. If a decision is introduced in messages 5-8, then modified in messages 23-25, list BOTH appearances with appropriate types.

EXAMPLE OUTPUT:
{
  "decisions": [
    {
      "tempId": "decision_1",
      "title": "Use file watching for conversation capture",
      "appearances": [
        {"messageStart": 12, "messageEnd": 14, "type": "introduced"},
        {"messageStart": 28, "messageEnd": 30, "type": "elaborated"}
      ],
      "confidence": 0.95
    },
    {
      "tempId": "decision_2",
      "title": "Two-phase extraction pipeline",
      "appearances": [
        {"messageStart": 18, "messageEnd": 22, "type": "introduced"}
      ],
      "confidence": 0.9
    }
  ]
}

CONVERSATION:
${transcript}`
}
```

**Step 2: Create extraction prompt**

Create `packages/extraction/src/prompts/extraction.ts`:

```typescript
import type { DecisionCandidate } from '../schemas.js'

export function buildExtractionPrompt(
  candidate: DecisionCandidate,
  contextMessages: string,
  otherCandidates: DecisionCandidate[]
): string {
  const otherDecisionsList = otherCandidates
    .filter((c) => c.tempId !== candidate.tempId)
    .map((c) => `- [${c.tempId}] ${c.title}`)
    .join('\n')

  const appearancesList = candidate.appearances
    .map((a) => `- Messages ${a.messageStart}-${a.messageEnd} (${a.type})`)
    .join('\n')

  return `You are extracting detailed information about a specific decision from a conversation.

DECISION: "${candidate.title}"
APPEARS AT:
${appearancesList}

RELEVANT CONTEXT (messages around each appearance):
${contextMessages}

OTHER DECISIONS IN THIS CONVERSATION (for dependency matching):
${otherDecisionsList || '(none)'}

Extract:
- summary: One paragraph explaining what was decided
- reasoning: The actual reasoning WHY this decision was made. Not just "they discussed it" but the specific logic, constraints, and considerations that led to this choice.
- alternativesConsidered: Array of alternatives that were discussed but not chosen
  - description: What the alternative was
  - whyRejected: Why it wasn't chosen
- status: One of:
  - "active": This is the current decision
  - "superseded": This was later replaced by another decision
  - "tentative": This was proposed but not firmly committed to
- dependsOn: Array of tempIds from other decisions that this decision depends on or builds upon. Only include clear dependencies, not vague relationships.
- confidence: 0-1 how confident you are in the accuracy of this extraction

EXAMPLE OUTPUT:
{
  "summary": "Use file watching on AI tool history directories rather than MCP for capture. The system will monitor ~/.claude/projects/ and similar directories for conversation files, parse them, and run extraction.",
  "reasoning": "MCP puts the AI tool in control of what gets sent to duplex stream. But the AI doesn't know what context will matter later - it would filter based on what seems relevant now. This creates the same problem as vibe coding: reasoning gets lost because the source decides what's important. By watching files instead, duplex stream controls capture and can extract everything, deciding importance through extraction rather than filtering at the source.",
  "alternativesConsidered": [
    {
      "description": "MCP server for both reading and writing - AI tools would push events to duplex stream",
      "whyRejected": "AI tool controls what gets captured, same filtering problem as vibe coding"
    },
    {
      "description": "Per-tool adapters using tool-specific APIs",
      "whyRejected": "Still relies on the tool to decide what to expose"
    }
  ],
  "status": "active",
  "dependsOn": ["decision_1"],
  "confidence": 0.95
}`
}
```

**Step 3: Create prompts index**

Create `packages/extraction/src/prompts/index.ts`:

```typescript
export * from './identification.js'
export * from './extraction.js'
```

**Step 4: Commit**

```bash
git add packages/extraction/src/prompts
git commit -m "feat(extraction): add prompt templates for decision extraction"
```

---

## Task 10: Create apps/api Worker Scaffold

**Files:**
- Create: `apps/api/package.json`
- Create: `apps/api/tsconfig.json`
- Create: `apps/api/wrangler.jsonc`
- Create: `apps/api/src/index.ts`
- Create: `apps/api/src/context.ts`

**Step 1: Create package.json**

```json
{
  "name": "api",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "bun runx dev",
    "build": "bun runx build",
    "deploy": "bun runx deploy",
    "check:types": "bun runx run-tsc",
    "check:lint": "bun runx run-eslint"
  },
  "dependencies": {
    "@ai-sdk/anthropic": "^1.0.0",
    "@hono/zod-validator": "^0.4.0",
    "@repo/db": "workspace:*",
    "@repo/extraction": "workspace:*",
    "@repo/hono-helpers": "workspace:*",
    "ai": "^4.0.0",
    "drizzle-orm": "^0.38.0",
    "hono": "catalog:",
    "zod": "catalog:"
  },
  "devDependencies": {
    "@cloudflare/vitest-pool-workers": "catalog:",
    "@cloudflare/workers-types": "catalog:",
    "@repo/eslint-config": "workspace:*",
    "@repo/tools": "workspace:*",
    "@repo/typescript-config": "workspace:*",
    "typescript": "catalog:",
    "vitest": "catalog:",
    "wrangler": "catalog:"
  }
}
```

**Step 2: Create tsconfig.json**

```json
{
  "extends": "@repo/typescript-config/workers.json",
  "compilerOptions": {
    "types": ["@cloudflare/workers-types/2023-07-01"]
  },
  "include": ["src/**/*", "*.config.ts", "worker-configuration.d.ts"]
}
```

**Step 3: Create wrangler.jsonc**

```jsonc
{
  "$schema": "node_modules/wrangler/config-schema.json",
  "name": "api",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-15",
  "compatibility_flags": ["nodejs_compat"],
  "observability": {
    "logs": {
      "enabled": true
    }
  },
  "ai": {
    "binding": "AI"
  },
  "d1_databases": [
    {
      "binding": "DB",
      "database_name": "duplex-dev",
      "database_id": "local"
    }
  ],
  "workflows": [
    {
      "name": "extract-conversation",
      "binding": "EXTRACT_WORKFLOW",
      "class_name": "ExtractConversationWorkflow"
    }
  ],
  "vars": {
    "ENVIRONMENT": "development"
  }
}
```

**Step 4: Create context.ts**

Create `apps/api/src/context.ts`:

```typescript
import type { DrizzleD1Database } from 'drizzle-orm/d1'
import type * as schema from '@repo/db/schema'

export interface Env {
  DB: D1Database
  AI: Ai
  EXTRACT_WORKFLOW: Workflow
  ANTHROPIC_API_KEY: string
  ENVIRONMENT: string
}

export interface HonoEnv {
  Bindings: Env
  Variables: {
    db: DrizzleD1Database<typeof schema>
    orgId: string
  }
}
```

**Step 5: Create index.ts**

Create `apps/api/src/index.ts`:

```typescript
import { Hono } from 'hono'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from './context.js'

const app = new Hono<HonoEnv>()

// Middleware: Database setup
app.use('*', async (c, next) => {
  const db = drizzle(c.env.DB, { schema })
  c.set('db', db)
  // Hardcoded dev org for now
  c.set('orgId', 'dev-org')
  await next()
})

// Health check
app.get('/', (c) => {
  return c.json({ status: 'ok', service: 'duplex-api' })
})

export default app
```

**Step 6: Install dependencies**

Run: `pnpm install`

**Step 7: Commit**

```bash
git add apps/api
git commit -m "feat(api): scaffold API worker with D1 and Drizzle"
```

---

## Task 11: Add Decision Query Routes

**Files:**
- Create: `apps/api/src/routes/decisions.ts`
- Modify: `apps/api/src/index.ts`

**Step 1: Create decisions routes**

Create `apps/api/src/routes/decisions.ts`:

```typescript
import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import { eq, and, or, like, desc } from 'drizzle-orm'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from '../context.js'

const app = new Hono<HonoEnv>()

// Search decisions
app.get(
  '/',
  zValidator(
    'query',
    z.object({
      q: z.string().min(1),
      workspaceId: z.string().optional(),
      limit: z.coerce.number().optional().default(20),
    })
  ),
  async (c) => {
    const { q, workspaceId, limit } = c.req.valid('query')
    const orgId = c.get('orgId')
    const db = c.get('db')

    const conditions = [
      eq(schema.decisions.orgId, orgId),
      or(
        like(schema.decisions.title, `%${q}%`),
        like(schema.decisions.summary, `%${q}%`),
        like(schema.decisions.reasoning, `%${q}%`)
      ),
    ]

    if (workspaceId) {
      conditions.push(eq(schema.decisions.workspaceId, workspaceId))
    }

    const decisions = await db.query.decisions.findMany({
      where: and(...conditions),
      with: {
        appearances: true,
        alternatives: true,
        dependencies: true,
      },
      orderBy: [desc(schema.decisions.confidence)],
      limit,
    })

    return c.json({ decisions })
  }
)

// Get single decision
app.get('/:id', async (c) => {
  const id = c.req.param('id')
  const db = c.get('db')

  const decision = await db.query.decisions.findFirst({
    where: eq(schema.decisions.id, id),
    with: {
      conversation: true,
      appearances: true,
      alternatives: true,
      dependencies: true,
    },
  })

  if (!decision) {
    return c.json({ error: 'Decision not found' }, 404)
  }

  return c.json({ decision })
})

export default app
```

**Step 2: Mount routes in index.ts**

Update `apps/api/src/index.ts`:

```typescript
import { Hono } from 'hono'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from './context.js'
import decisionsRoutes from './routes/decisions.js'

const app = new Hono<HonoEnv>()

// Middleware: Database setup
app.use('*', async (c, next) => {
  const db = drizzle(c.env.DB, { schema })
  c.set('db', db)
  c.set('orgId', 'dev-org')
  await next()
})

// Health check
app.get('/', (c) => {
  return c.json({ status: 'ok', service: 'duplex-api' })
})

// Routes
app.route('/decisions', decisionsRoutes)

export default app
```

**Step 3: Commit**

```bash
git add apps/api/src/routes apps/api/src/index.ts
git commit -m "feat(api): add decision query routes"
```

---

## Task 12: Add Extraction Route (Trigger Workflow)

**Files:**
- Create: `apps/api/src/routes/extraction.ts`
- Modify: `apps/api/src/index.ts`

**Step 1: Create extraction routes**

Create `apps/api/src/routes/extraction.ts`:

```typescript
import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import type { HonoEnv } from '../context.js'

const app = new Hono<HonoEnv>()

// Trigger extraction workflow
app.post(
  '/conversations/extract',
  zValidator(
    'json',
    z.object({
      content: z.string(),
      sourcePath: z.string(),
      source: z.enum(['claude-code', 'claude-web', 'cursor', 'other']),
      workspaceId: z.string(),
    })
  ),
  async (c) => {
    const { content, sourcePath, source, workspaceId } = c.req.valid('json')
    const orgId = c.get('orgId')

    const instance = await c.env.EXTRACT_WORKFLOW.create({
      params: { orgId, workspaceId, content, sourcePath, source },
    })

    return c.json({
      workflowId: instance.id,
      status: 'started',
    })
  }
)

// Get workflow status
app.get('/workflows/:id', async (c) => {
  const id = c.req.param('id')

  const instance = await c.env.EXTRACT_WORKFLOW.get(id)
  const status = await instance.status()

  return c.json({
    id,
    status: status.status,
    output: status.output,
    error: status.error,
  })
})

export default app
```

**Step 2: Mount routes in index.ts**

Update `apps/api/src/index.ts` to add:

```typescript
import extractionRoutes from './routes/extraction.js'

// Add after decisionsRoutes
app.route('/extraction', extractionRoutes)
```

**Step 3: Commit**

```bash
git add apps/api/src/routes/extraction.ts apps/api/src/index.ts
git commit -m "feat(api): add extraction trigger route"
```

---

## Task 13: Create Extraction Workflow

**Files:**
- Create: `apps/api/src/workflows/extract-conversation.ts`
- Modify: `apps/api/src/index.ts` (export workflow)

**Step 1: Create workflow**

Create `apps/api/src/workflows/extract-conversation.ts`:

```typescript
import {
  WorkflowEntrypoint,
  WorkflowStep,
  WorkflowEvent,
} from 'cloudflare:workers'
import { createAnthropic } from '@ai-sdk/anthropic'
import { generateObject } from 'ai'
import { parseClaudeCodeSession } from '@repo/extraction/parsers'
import { buildTranscript, buildContextWindow } from '@repo/extraction'
import {
  buildIdentificationPrompt,
  buildExtractionPrompt,
} from '@repo/extraction/prompts'
import {
  IdentificationResponseSchema,
  ExtractedDecisionSchema,
  type DecisionCandidate,
} from '@repo/extraction'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { Env } from '../context.js'

interface ExtractConversationParams {
  orgId: string
  workspaceId: string
  content: string
  sourcePath: string
  source: 'claude-code' | 'claude-web' | 'cursor' | 'other'
}

interface ExtractConversationResult {
  conversationId: string
  decisionCount: number
}

export class ExtractConversationWorkflow extends WorkflowEntrypoint<
  Env,
  ExtractConversationParams,
  ExtractConversationResult
> {
  async run(
    event: WorkflowEvent<ExtractConversationParams>,
    step: WorkflowStep
  ): Promise<ExtractConversationResult> {
    const { orgId, workspaceId, content, sourcePath, source } = event.payload

    // Create AI client
    const baseURL = await this.env.AI.gateway('duplex-extraction').getUrl(
      'anthropic'
    )
    const client = createAnthropic({
      apiKey: this.env.ANTHROPIC_API_KEY,
      baseURL,
    })

    // Step 1: Parse content
    const conversation = await step.do('parse-content', async () => {
      switch (source) {
        case 'claude-code':
          return parseClaudeCodeSession(content, { sourcePath })
        default:
          throw new Error(`Parser not implemented for source: ${source}`)
      }
    })

    // Step 2: Build transcript
    const transcript = await step.do('build-transcript', async () => {
      return buildTranscript(conversation.messages)
    })

    // Step 3: Identify decisions (Phase 1)
    const candidates = await step.do('identify-decisions', async () => {
      const { object } = await generateObject({
        model: client('claude-sonnet-4-5-20250929'),
        schema: IdentificationResponseSchema,
        prompt: buildIdentificationPrompt(transcript),
      })
      return object.decisions
    })

    // Step 4: Extract each decision (Phase 2)
    const extracted: Array<
      DecisionCandidate & { summary: string; reasoning: string; status: string; dependsOn: string[]; alternativesConsidered: Array<{ description: string; whyRejected: string }> }
    > = []

    for (const [i, candidate] of candidates.entries()) {
      const decision = await step.do(`extract-decision-${i}`, async () => {
        const contextMessages = buildContextWindow(
          conversation.messages,
          candidate.appearances,
          { buffer: 2 }
        )

        const { object } = await generateObject({
          model: client('claude-sonnet-4-5-20250929'),
          schema: ExtractedDecisionSchema,
          prompt: buildExtractionPrompt(candidate, contextMessages, candidates),
        })

        return {
          ...candidate,
          ...object,
        }
      })

      extracted.push(decision)
    }

    // Step 5: Store results
    const result = await step.do('store-results', async () => {
      const db = drizzle(this.env.DB, { schema })
      const conversationId = crypto.randomUUID()
      const now = new Date().toISOString()

      // Build ID mapping
      const idMap = new Map<string, string>()
      for (const d of extracted) {
        idMap.set(d.tempId, crypto.randomUUID())
      }

      // Insert conversation
      await db.insert(schema.conversations).values({
        id: conversationId,
        orgId,
        workspaceId,
        source,
        sourcePath,
        projectPath: conversation.projectPath,
        sessionId: conversation.sessionId,
        messageCount: conversation.messages.length,
        createdAt: conversation.createdAt.toISOString(),
        extractedAt: now,
      })

      // Insert messages
      for (const msg of conversation.messages) {
        await db.insert(schema.messages).values({
          id: crypto.randomUUID(),
          conversationId,
          index: msg.index,
          role: msg.role,
          content: msg.content,
          thinking: msg.thinking,
          timestamp: msg.timestamp?.toISOString(),
        })
      }

      // Insert decisions and related data
      for (const d of extracted) {
        const decisionId = idMap.get(d.tempId)!

        await db.insert(schema.decisions).values({
          id: decisionId,
          conversationId,
          orgId,
          workspaceId,
          title: d.title,
          summary: d.summary,
          reasoning: d.reasoning,
          status: d.status as 'active' | 'superseded' | 'tentative',
          confidence: d.confidence,
          extractedAt: now,
        })

        // Insert appearances
        for (const app of d.appearances) {
          await db.insert(schema.decisionAppearances).values({
            id: crypto.randomUUID(),
            decisionId,
            messageStart: app.messageStart,
            messageEnd: app.messageEnd,
            type: app.type,
          })
        }

        // Insert alternatives
        for (const alt of d.alternativesConsidered) {
          await db.insert(schema.alternatives).values({
            id: crypto.randomUUID(),
            decisionId,
            description: alt.description,
            whyRejected: alt.whyRejected,
          })
        }

        // Insert dependencies
        for (const depTempId of d.dependsOn) {
          const resolvedId = idMap.get(depTempId)
          await db.insert(schema.decisionDependencies).values({
            id: crypto.randomUUID(),
            fromDecisionId: decisionId,
            toDecisionRef: resolvedId
              ? `decision:${resolvedId}`
              : depTempId,
          })
        }
      }

      return { conversationId }
    })

    return {
      conversationId: result.conversationId,
      decisionCount: extracted.length,
    }
  }
}
```

**Step 2: Export workflow from index.ts**

Update `apps/api/src/index.ts` to add at the end:

```typescript
export { ExtractConversationWorkflow } from './workflows/extract-conversation.js'
```

**Step 3: Commit**

```bash
git add apps/api/src/workflows apps/api/src/index.ts
git commit -m "feat(api): add extraction workflow with two-phase LLM"
```

---

## Task 14: Create Dev Test Script

**Files:**
- Create: `scripts/test-extraction.ts`

**Step 1: Create test script**

Create `scripts/test-extraction.ts`:

```typescript
#!/usr/bin/env bun

import { readFile } from 'fs/promises'
import { parseClaudeCodeSession } from '../packages/extraction/src/parsers/claude-code.js'
import { buildTranscript } from '../packages/extraction/src/transcript.js'

async function main() {
  const sessionPath = process.argv[2]

  if (!sessionPath) {
    console.error('Usage: bun scripts/test-extraction.ts <session-file.jsonl>')
    process.exit(1)
  }

  console.log(`Reading: ${sessionPath}`)
  const content = await readFile(sessionPath, 'utf-8')

  console.log('Parsing...')
  const conversation = parseClaudeCodeSession(content, { sourcePath: sessionPath })

  console.log(`Session ID: ${conversation.sessionId}`)
  console.log(`Project: ${conversation.projectPath}`)
  console.log(`Messages: ${conversation.messages.length}`)
  console.log(`Created: ${conversation.createdAt.toISOString()}`)

  console.log('\n--- Message Summary ---')
  const userCount = conversation.messages.filter(m => m.role === 'user').length
  const assistantCount = conversation.messages.filter(m => m.role === 'assistant').length
  const systemCount = conversation.messages.filter(m => m.role === 'system').length
  console.log(`User: ${userCount}, Assistant: ${assistantCount}, System: ${systemCount}`)

  console.log('\n--- Transcript Preview (first 2000 chars) ---')
  const transcript = buildTranscript(conversation.messages)
  console.log(transcript.slice(0, 2000))
  console.log('...')

  console.log(`\nTotal transcript length: ${transcript.length} chars`)
}

main().catch(console.error)
```

**Step 2: Commit**

```bash
git add scripts/test-extraction.ts
git commit -m "feat: add dev script to test conversation parsing"
```

---

## Task 15: Test Parser on Current Conversation

**Step 1: Run test script**

Run: `bun scripts/test-extraction.ts ~/.claude/projects/-Users-asnodgrass-lil-duplex-stream-duplex-stream/1a3d2f9d-6ad6-46f1-9082-7659be91a183.jsonl`

Expected: Output showing parsed conversation with message counts and transcript preview.

**Step 2: Verify output looks correct**

Check that:
- Session ID matches
- Message counts are reasonable
- Transcript includes both content and thinking blocks

---

## Summary

This plan creates the core extraction pipeline:

1. **packages/db** - Drizzle schema and types for conversations, decisions, relationships
2. **packages/extraction** - Parsers, prompts, transcript utilities, Zod schemas
3. **apps/api** - Hono worker with D1, decision query routes, extraction workflow

After implementation, you can:
1. Parse Claude Code sessions locally with the test script
2. Trigger extraction via API (requires deployed worker + Anthropic key)
3. Query extracted decisions via API

**Next steps after this plan:**
- Deploy to Cloudflare
- Set up AI Gateway named "duplex-extraction"
- Add ANTHROPIC_API_KEY secret
- Run extraction on this conversation
- Evaluate extraction quality
