import type { DrizzleD1Database } from 'drizzle-orm/d1'
import type * as schema from '@repo/db/schema'

export interface Env {
	DB: D1Database
	AI: Ai
	EXTRACT_WORKFLOW: Workflow
	ANTHROPIC_API_KEY?: string // Optional when using AI Gateway provider tokens
	ENVIRONMENT: string
	WORKOS_CLIENT_ID: string // WorkOS client ID for JWT validation
}

export interface HonoEnv {
	Bindings: Env
	Variables: {
		db: DrizzleD1Database<typeof schema>
		userId: string
		orgId: string
		permissions: string[]
	}
}
