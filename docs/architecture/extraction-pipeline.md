# Extraction Pipeline Architecture

> **Status**: Design complete, ready for implementation
> **Created**: 2026-01-30
> **Last Updated**: 2026-01-30

## Overview

The extraction pipeline captures reasoning from AI coding conversations (Claude Code, Claude Web, Cursor, etc.) and extracts structured decisions with their reasoning chains and dependencies.

### Goals

1. Parse conversation files from multiple AI tools
2. Extract decisions with reasoning, alternatives, and dependencies
3. Store in queryable format (D1)
4. Enable queries like "Why did we choose X over Y?"

### Success Criteria

Import a conversation. Query "why did we choose file watching over MCP for capture?" and get back:
- **Decision**: Use file watching on AI tool history directories, not MCP
- **Reasoning**: MCP puts the AI tool in control of what gets sent. AI doesn't know what context will matter later.
- **Depends on**: Core principle that we don't trust AI to decide what's relevant
- **Alternatives considered**: MCP for both read/write, per-tool adapters

---

## Architecture

### Two-Phase Extraction

**Phase 1: Decision Identification**
- Parse full conversation into structured transcript
- Send to LLM: "Identify all decisions made"
- Output: Decision candidates with location pointers (non-contiguous)

**Phase 2: Decision Extraction**
- For each candidate, build context window from appearances
- Send to LLM: "Extract full reasoning for this decision"
- Output: Complete decision records with dependencies

### Why Two Phases?

- Phase 1 sees full conversation but outputs only pointers (handles long conversations)
- Phase 2 focuses on relevant sections (manageable context per extraction)
- No information loss from early in thread—Phase 1 scans everything

---

## Data Sources

### Claude Code Sessions

Location: `~/.claude/projects/{project-path-encoded}/{session-id}.jsonl`

Format: JSONL with typed events
```jsonl
{"type": "user", "message": {"role": "user", "content": "..."}, ...}
{"type": "assistant", "message": {"role": "assistant", "content": [...]}, ...}
{"type": "progress", "data": {...}, ...}
{"type": "system", ...}
```

Key fields:
- `type`: Event type (`user`, `assistant`, `progress`, `system`, `file-history-snapshot`, `summary`)
- `message.content`: String for user, array for assistant (includes `thinking`, `text`, `tool_use` blocks)
- `sessionId`: Session identifier
- `timestamp`: ISO timestamp

### Claude Web Export (Future)

Manual JSON export from claude.ai. Different format, complementary content (architecture decisions, design discussions).

---

## Database Schema

### Drizzle Schema (`packages/db/schema/extraction.ts`)

```typescript
import { sqliteTable, text, integer, real } from 'drizzle-orm/sqlite-core'
import { relations } from 'drizzle-orm'

// Conversations table
export const conversations = sqliteTable('conversations', {
  id: text('id').primaryKey(),
  orgId: text('org_id').notNull(),              // Database is per-org
  workspaceId: text('workspace_id').notNull(),  // Filter lens within org
  source: text('source', {
    enum: ['claude-code', 'claude-web', 'cursor', 'other']
  }).notNull(),
  sourcePath: text('source_path').notNull(),
  projectPath: text('project_path').notNull(),
  sessionId: text('session_id'),                // Original session ID
  messageCount: integer('message_count').notNull(),
  createdAt: text('created_at').notNull(),      // Original conversation timestamp
  extractedAt: text('extracted_at'),            // When extraction ran
})

// Messages table
export const messages = sqliteTable('messages', {
  id: text('id').primaryKey(),
  conversationId: text('conversation_id').notNull()
    .references(() => conversations.id, { onDelete: 'cascade' }),
  index: integer('index').notNull(),
  role: text('role', { enum: ['user', 'assistant', 'system'] }).notNull(),
  content: text('content').notNull(),
  thinking: text('thinking'),                   // Assistant thinking block
  timestamp: text('timestamp'),
})

// Decisions table
export const decisions = sqliteTable('decisions', {
  id: text('id').primaryKey(),
  conversationId: text('conversation_id').notNull()
    .references(() => conversations.id, { onDelete: 'cascade' }),
  orgId: text('org_id').notNull(),              // Denormalized for query efficiency
  workspaceId: text('workspace_id').notNull(),
  title: text('title').notNull(),
  summary: text('summary').notNull(),
  reasoning: text('reasoning').notNull(),
  status: text('status', {
    enum: ['active', 'superseded', 'tentative']
  }).notNull(),
  confidence: real('confidence').notNull(),     // 0-1, extraction confidence
  extractedAt: text('extracted_at').notNull(),
})

// Decision appearances (non-contiguous locations)
export const decisionAppearances = sqliteTable('decision_appearances', {
  id: text('id').primaryKey(),
  decisionId: text('decision_id').notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  messageStart: integer('message_start').notNull(),
  messageEnd: integer('message_end').notNull(),
  type: text('type', {
    enum: ['introduced', 'elaborated', 'modified', 'reaffirmed']
  }).notNull(),
  context: text('context'),
})

// Alternatives considered
export const alternatives = sqliteTable('alternatives', {
  id: text('id').primaryKey(),
  decisionId: text('decision_id').notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  description: text('description').notNull(),
  whyRejected: text('why_rejected').notNull(),
})

// Decision dependencies (graph edges)
export const decisionDependencies = sqliteTable('decision_dependencies', {
  id: text('id').primaryKey(),
  fromDecisionId: text('from_decision_id').notNull()
    .references(() => decisions.id, { onDelete: 'cascade' }),
  toDecisionRef: text('to_decision_ref').notNull(),
  // URI format: "decision:xyz" (same conversation) or "conversation:abc/decision:xyz"
})

// Relations for Drizzle query builder
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

export const decisionAppearancesRelations = relations(decisionAppearances, ({ one }) => ({
  decision: one(decisions, {
    fields: [decisionAppearances.decisionId],
    references: [decisions.id],
  }),
}))

export const alternativesRelations = relations(alternatives, ({ one }) => ({
  decision: one(decisions, {
    fields: [alternatives.decisionId],
    references: [decisions.id],
  }),
}))

export const decisionDependenciesRelations = relations(decisionDependencies, ({ one }) => ({
  fromDecision: one(decisions, {
    fields: [decisionDependencies.fromDecisionId],
    references: [decisions.id],
  }),
}))
```

### Type Derivation (`packages/db/types.ts`)

```typescript
import type { InferSelectModel, InferInsertModel } from 'drizzle-orm'
import * as schema from './schema/extraction'

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

---

## LLM Integration

### Stack

- **Model**: Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`)
- **Gateway**: Cloudflare AI Gateway (logging, caching, rate limiting)
- **SDK**: Vercel AI SDK (streaming, structured output)

### Configuration

```typescript
// packages/extraction/src/ai.ts
import { createAnthropic } from '@ai-sdk/anthropic'

// Uses Cloudflare AI binding for gateway URL resolution
// Requires AI binding in wrangler.jsonc: { "ai": { "binding": "AI" } }

export async function createExtractionClient(env: Env) {
  const baseURL = await env.AI.gateway('duplex-extraction').getUrl('anthropic')

  return createAnthropic({
    apiKey: env.ANTHROPIC_API_KEY,
    baseURL,
  })
}
```

### Wrangler AI Binding

```jsonc
// apps/api/wrangler.jsonc
{
  "ai": {
    "binding": "AI"
  }
}
```

### LLM Call Pattern

```typescript
import { generateObject } from 'ai'
import { z } from 'zod'
import { createExtractionClient } from './ai'

// Phase 1: Identify decisions
const DecisionCandidateSchema = z.object({
  tempId: z.string(),
  title: z.string(),
  appearances: z.array(z.object({
    messageStart: z.number(),
    messageEnd: z.number(),
    type: z.enum(['introduced', 'elaborated', 'modified', 'reaffirmed']),
  })),
  confidence: z.number().min(0).max(1),
})

const IdentificationResponseSchema = z.object({
  decisions: z.array(DecisionCandidateSchema),
})

async function identifyDecisions(
  client: ReturnType<typeof createExtractionClient>,
  transcript: string
): Promise<z.infer<typeof IdentificationResponseSchema>> {
  const { object } = await generateObject({
    model: client('claude-sonnet-4-5-20250929'),
    schema: IdentificationResponseSchema,
    prompt: buildIdentificationPrompt(transcript),
  })
  return object
}

// Phase 2: Extract decision details
const ExtractedDecisionSchema = z.object({
  summary: z.string(),
  reasoning: z.string(),
  alternativesConsidered: z.array(z.object({
    description: z.string(),
    whyRejected: z.string(),
  })),
  status: z.enum(['active', 'superseded', 'tentative']),
  dependsOn: z.array(z.string()),  // tempIds
  confidence: z.number().min(0).max(1),
})

async function extractDecision(
  client: ReturnType<typeof createExtractionClient>,
  candidate: DecisionCandidate,
  contextMessages: string,
  otherCandidates: DecisionCandidate[]
): Promise<z.infer<typeof ExtractedDecisionSchema>> {
  const { object } = await generateObject({
    model: client('claude-sonnet-4-5-20250929'),
    schema: ExtractedDecisionSchema,
    prompt: buildExtractionPrompt(candidate, contextMessages, otherCandidates),
  })
  return object
}
```

---

## Prompt Templates

### Phase 1: Decision Identification

```typescript
function buildIdentificationPrompt(transcript: string): string {
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

### Phase 2: Decision Extraction

```typescript
function buildExtractionPrompt(
  candidate: DecisionCandidate,
  contextMessages: string,
  otherCandidates: DecisionCandidate[]
): string {
  const otherDecisionsList = otherCandidates
    .filter(c => c.tempId !== candidate.tempId)
    .map(c => `- [${c.tempId}] ${c.title}`)
    .join('\n')

  const appearancesList = candidate.appearances
    .map(a => `- Messages ${a.messageStart}-${a.messageEnd} (${a.type})`)
    .join('\n')

  return `You are extracting detailed information about a specific decision from a conversation.

DECISION: "${candidate.title}"
APPEARS AT:
${appearancesList}

RELEVANT CONTEXT (messages around each appearance):
${contextMessages}

OTHER DECISIONS IN THIS CONVERSATION (for dependency matching):
${otherDecisionsList}

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

---

## Cloudflare Workflow

### Workflow Definition (`apps/api/src/workflows/extract-conversation.ts`)

```typescript
import { WorkflowEntrypoint, WorkflowStep, WorkflowEvent } from 'cloudflare:workers'
import { createExtractionClient } from '@repo/extraction/ai'
import { parseClaudeCodeSession } from '@repo/extraction/parsers/claude-code'
import { buildTranscript, buildContextWindow } from '@repo/extraction/transcript'
import { buildIdentificationPrompt, buildExtractionPrompt } from '@repo/extraction/prompts'
import { storeConversation } from '@repo/db/mutations'
import { generateObject } from 'ai'

interface ExtractConversationParams {
  orgId: string
  workspaceId: string
  /** File content as string (file reading happens outside workflow) */
  content: string
  /** Original file path for metadata extraction */
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
    const client = await createExtractionClient(this.env)

    // Step 1: Parse content into messages
    const conversation = await step.do('parse-content', async () => {
      switch (source) {
        case 'claude-code':
          return parseClaudeCodeSession(content, { sourcePath })
        case 'claude-web':
          // TODO: Implement
          throw new Error('Claude web parser not implemented')
        default:
          throw new Error(`Unknown source: ${source}`)
      }
    })

    // Step 2: Build transcript with thinking blocks
    const transcript = await step.do('build-transcript', async () => {
      return buildTranscript(conversation.messages)
    })

    // Step 3: Identify decisions (Phase 1 LLM)
    const candidates = await step.do('identify-decisions', async () => {
      const { object } = await generateObject({
        model: client('claude-sonnet-4-5-20250929'),
        schema: IdentificationResponseSchema,
        prompt: buildIdentificationPrompt(transcript),
      })
      return object.decisions
    })

    // Step 4: Extract each decision (Phase 2 LLM)
    // Each extraction is a separate durable step
    const extracted: ExtractedDecision[] = []

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
          ...object,
          tempId: candidate.tempId,
          title: candidate.title,
          appearances: candidate.appearances,
        }
      })

      extracted.push(decision)
    }

    // Step 5: Resolve dependencies and store
    const result = await step.do('store-results', async () => {
      // Build tempId -> UUID mapping
      const idMap = new Map<string, string>()
      for (const decision of extracted) {
        idMap.set(decision.tempId, crypto.randomUUID())
      }

      // Resolve dependency references
      const resolved = extracted.map(d => ({
        ...d,
        id: idMap.get(d.tempId)!,
        dependsOn: d.dependsOn.map(tempId => {
          const resolvedId = idMap.get(tempId)
          // Same-conversation reference
          if (resolvedId) return `decision:${resolvedId}`
          // Cross-conversation reference (already in URI format)
          return tempId
        }),
      }))

      return storeConversation(this.env.DB, {
        orgId,
        workspaceId,
        source,
        sourcePath,
        conversation,
        decisions: resolved,
      })
    })

    return {
      conversationId: result.conversationId,
      decisionCount: extracted.length
    }
  }
}
```

### Wrangler Binding

```jsonc
// apps/api/wrangler.jsonc
{
  "workflows": [
    {
      "name": "extract-conversation",
      "binding": "EXTRACT_WORKFLOW",
      "class_name": "ExtractConversationWorkflow"
    }
  ]
}
```

---

## Parsers

### Design Note: File Reading Outside Workers

Parsers accept file **content** as a string, not file paths. File reading happens outside Workers:
- **Dev/testing**: CLI script reads files locally, calls extraction API with content
- **Production**: Files uploaded to R2, workflow reads from R2

This avoids Node.js `fs` in Workers runtime.

### Claude Code Parser (`packages/extraction/src/parsers/claude-code.ts`)

```typescript
// Parser accepts content string - file reading happens outside Workers

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
  /** Original file path (for metadata extraction, not file reading) */
  sourcePath?: string
}

/**
 * Parse Claude Code session JSONL content into structured conversation.
 * @param content - JSONL file content as string
 * @param options - Optional metadata like original file path
 */
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

      // Extract session metadata
      if (event.sessionId && !sessionId) {
        sessionId = event.sessionId
      }

      // Track earliest timestamp
      if (event.timestamp) {
        const ts = new Date(event.timestamp)
        if (!earliestTimestamp || ts < earliestTimestamp) {
          earliestTimestamp = ts
        }
      }

      // Parse user messages
      if (event.type === 'user' && event.message) {
        messages.push({
          index: messages.length,
          role: 'user',
          content: typeof event.message.content === 'string'
            ? event.message.content
            : '',
          timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
        })
      }

      // Parse assistant messages
      if (event.type === 'assistant' && event.message) {
        const contentArray = Array.isArray(event.message.content)
          ? event.message.content
          : []

        // Extract text content
        const textContent = contentArray
          .filter(block => block.type === 'text')
          .map(block => block.text || '')
          .join('\n')

        // Extract thinking content
        const thinkingContent = contentArray
          .filter(block => block.type === 'thinking')
          .map(block => block.thinking || '')
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

      // Parse system messages
      if (event.type === 'system' && event.message) {
        messages.push({
          index: messages.length,
          role: 'system',
          content: typeof event.message.content === 'string'
            ? event.message.content
            : JSON.stringify(event.message.content),
          timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
        })
      }
    } catch (e) {
      // Skip malformed lines
      continue
    }
  }

  // Extract project path from source path if provided
  // ~/.claude/projects/-Users-asnodgrass-lil-duplex-stream-duplex-stream/session.jsonl
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

### Transcript Builder (`packages/extraction/src/transcript.ts`)

```typescript
import type { ParsedMessage } from './parsers/claude-code'

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
  buffer?: number  // Messages before/after each appearance
}

export function buildContextWindow(
  messages: ParsedMessage[],
  appearances: Appearance[],
  options: ContextWindowOptions = {}
): string {
  const { buffer = 2 } = options

  // Collect all message indices we need
  const indices = new Set<number>()

  for (const appearance of appearances) {
    const start = Math.max(0, appearance.messageStart - buffer)
    const end = Math.min(messages.length - 1, appearance.messageEnd + buffer)

    for (let i = start; i <= end; i++) {
      indices.add(i)
    }
  }

  // Sort and build transcript for just those messages
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

---

## Query Layer

### Decision Queries (`packages/db/queries/decisions.ts`)

```typescript
import { eq, and, or, like, desc, sql } from 'drizzle-orm'
import type { DrizzleD1Database } from 'drizzle-orm/d1'
import * as schema from '../schema/extraction'
import type { DecisionWithRelations } from '../types'

export function createDecisionQueries(db: DrizzleD1Database<typeof schema>) {
  return {
    // Get decision with full relations
    async getDecision(decisionId: string): Promise<DecisionWithRelations | null> {
      const result = await db.query.decisions.findFirst({
        where: eq(schema.decisions.id, decisionId),
        with: {
          conversation: true,
          appearances: true,
          alternatives: true,
          dependencies: true,
        },
      })
      return result ?? null
    },

    // Search decisions by text
    async searchDecisions(params: {
      orgId: string
      workspaceId?: string
      query: string
      limit?: number
    }): Promise<DecisionWithRelations[]> {
      const { orgId, workspaceId, query, limit = 20 } = params

      const conditions = [
        eq(schema.decisions.orgId, orgId),
        or(
          like(schema.decisions.title, `%${query}%`),
          like(schema.decisions.summary, `%${query}%`),
          like(schema.decisions.reasoning, `%${query}%`)
        ),
      ]

      if (workspaceId) {
        conditions.push(eq(schema.decisions.workspaceId, workspaceId))
      }

      return db.query.decisions.findMany({
        where: and(...conditions),
        with: {
          appearances: true,
          alternatives: true,
          dependencies: true,
        },
        orderBy: [desc(schema.decisions.confidence)],
        limit,
      })
    },

    // Get dependency graph
    async getDecisionGraph(
      decisionId: string,
      depth: number = 2
    ): Promise<{
      nodes: DecisionWithRelations[]
      edges: Array<{ from: string; to: string }>
    }> {
      const visited = new Set<string>()
      const nodes: DecisionWithRelations[] = []
      const edges: Array<{ from: string; to: string }> = []

      const traverse = async (id: string, currentDepth: number) => {
        if (visited.has(id) || currentDepth > depth) return
        visited.add(id)

        const decision = await this.getDecision(id)
        if (!decision) return

        nodes.push(decision)

        for (const dep of decision.dependencies) {
          edges.push({ from: id, to: dep.toDecisionRef })

          // Extract decision ID from URI for traversal
          const match = dep.toDecisionRef.match(/decision:([^/]+)$/)
          if (match) {
            await traverse(match[1], currentDepth + 1)
          }
        }
      }

      await traverse(decisionId, 0)
      return { nodes, edges }
    },

    // List decisions for a conversation
    async getConversationDecisions(
      conversationId: string
    ): Promise<DecisionWithRelations[]> {
      return db.query.decisions.findMany({
        where: eq(schema.decisions.conversationId, conversationId),
        with: {
          appearances: true,
          alternatives: true,
          dependencies: true,
        },
        orderBy: [desc(schema.decisions.confidence)],
      })
    },
  }
}
```

---

## API Routes

### Decision Routes (`apps/api/src/routes/decisions.ts`)

```typescript
import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import { createDecisionQueries } from '@repo/db/queries/decisions'
import type { HonoEnv } from '../types'

const app = new Hono<HonoEnv>()

// Search decisions
app.get(
  '/',
  zValidator('query', z.object({
    q: z.string().min(1),
    workspaceId: z.string().optional(),
    limit: z.coerce.number().optional(),
  })),
  async (c) => {
    const { q, workspaceId, limit } = c.req.valid('query')
    const orgId = c.get('orgId')
    const queries = createDecisionQueries(c.get('db'))

    const decisions = await queries.searchDecisions({
      orgId,
      workspaceId,
      query: q,
      limit,
    })

    return c.json({ decisions })
  }
)

// Get decision with graph
app.get('/:id', async (c) => {
  const id = c.req.param('id')
  const queries = createDecisionQueries(c.get('db'))

  const decision = await queries.getDecision(id)
  if (!decision) {
    return c.json({ error: 'Decision not found' }, 404)
  }

  const graph = await queries.getDecisionGraph(id)

  return c.json({ decision, graph })
})

export default app
```

### Extraction Routes (`apps/api/src/routes/extraction.ts`)

```typescript
import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import type { HonoEnv } from '../types'

const app = new Hono<HonoEnv>()

// Trigger extraction workflow
app.post(
  '/conversations/extract',
  zValidator('json', z.object({
    content: z.string(),           // File content (reading happens client-side)
    sourcePath: z.string(),        // Original file path for metadata
    source: z.enum(['claude-code', 'claude-web', 'cursor', 'other']),
    workspaceId: z.string(),
  })),
  async (c) => {
    const { content, sourcePath, source, workspaceId } = c.req.valid('json')
    const orgId = c.get('orgId')

    const instance = await c.env.EXTRACT_WORKFLOW.create({
      params: { orgId, workspaceId, content, sourcePath, source }
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

---

## Package Structure

```
packages/
├── db/
│   ├── src/
│   │   ├── schema/
│   │   │   └── extraction.ts      # Drizzle schema
│   │   ├── queries/
│   │   │   └── decisions.ts       # Query functions
│   │   ├── mutations/
│   │   │   └── conversations.ts   # Insert functions
│   │   ├── types.ts               # Derived types
│   │   ├── client.ts              # D1 client setup
│   │   └── index.ts
│   ├── drizzle.config.ts
│   └── package.json
│
├── extraction/
│   ├── src/
│   │   ├── parsers/
│   │   │   ├── claude-code.ts     # Claude Code JSONL parser
│   │   │   └── claude-web.ts      # Claude Web JSON parser (future)
│   │   ├── prompts/
│   │   │   ├── identification.ts  # Phase 1 prompt
│   │   │   └── extraction.ts      # Phase 2 prompt
│   │   ├── schemas/
│   │   │   └── llm-responses.ts   # Zod schemas for LLM output
│   │   ├── transcript.ts          # Transcript building utilities
│   │   ├── ai.ts                  # AI Gateway client setup
│   │   └── index.ts
│   └── package.json

apps/
├── api/
│   ├── src/
│   │   ├── routes/
│   │   │   ├── decisions.ts
│   │   │   ├── extraction.ts
│   │   │   └── index.ts
│   │   ├── workflows/
│   │   │   └── extract-conversation.ts
│   │   ├── middleware/
│   │   │   └── auth.ts            # Hardcoded dev user for now
│   │   └── index.ts
│   ├── wrangler.jsonc
│   └── package.json
```

---

## Implementation Order

1. **`packages/db`**: Schema, types, migrations
2. **`packages/extraction`**: Parsers, prompts, AI client
3. **`apps/api`**: Routes, workflow, dev auth middleware
4. **Test extraction**: Run on this conversation, evaluate quality
5. **Iterate**: Adjust prompts based on extraction results

---

## Environment Variables

```bash
# Cloudflare
CF_ACCOUNT_ID=your-account-id
CLOUDFLARE_API_TOKEN=your-token

# Anthropic (via AI Gateway)
ANTHROPIC_API_KEY=your-key

# Development
ENVIRONMENT=development
```

---

## Open Questions

1. **Chunking strategy**: When conversations exceed context window, how do we split while preserving decision continuity?
2. **Vectorize integration**: When do we add semantic search? After proving text search works?
3. **Cross-conversation linking**: How do we detect when a decision in conversation B references one from conversation A?
4. **Extraction triggers**: File watcher vs manual trigger vs scheduled scan?

---

## Next Steps

1. Create `packages/db` with Drizzle schema
2. Create `packages/extraction` with Claude Code parser
3. Create `apps/api` with extraction workflow
4. Run extraction on this conversation
5. Evaluate output against success criteria
