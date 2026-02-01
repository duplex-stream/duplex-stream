import type { DrizzleD1Database } from 'drizzle-orm/d1'
import type * as schema from '@repo/db/schema'

export interface Env {
	DB: D1Database
	AI: Ai
	EXTRACT_WORKFLOW: Workflow
	CONVERSATIONS_BUCKET: R2Bucket // R2 bucket for large conversation uploads
	ANTHROPIC_API_KEY: string // Anthropic API key - required for API calls
	ENVIRONMENT: string
	WORKOS_CLIENT_ID: string // WorkOS client ID for JWT validation
	CF_ACCOUNT_ID: string // Cloudflare account ID for AI Gateway
	AI_GATEWAY_NAME: string // AI Gateway name for routing requests
	CF_AIG_TOKEN?: string // Optional: AI Gateway auth token (for authenticated gateways)
	R2_ACCESS_KEY_ID: string // R2 access key for presigned URLs
	R2_SECRET_ACCESS_KEY: string // R2 secret key for presigned URLs
	R2_BUCKET_NAME: string // R2 bucket name
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
