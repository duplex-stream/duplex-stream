import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3'
import { getSignedUrl } from '@aws-sdk/s3-request-presigner'
import type { HonoEnv } from '../context'

const app = new Hono<HonoEnv>()

// Upload URL schema for requesting presigned R2 URL
const uploadUrlSchema = z.object({
	filename: z.string(),
	contentHash: z.string(),
	source: z.enum(['claude-code', 'claude-web', 'cursor', 'other']),
	workspaceId: z.string(),
})

// Base fields for extraction
const extractBaseSchema = z.object({
	sourcePath: z.string(),
	source: z.enum(['claude-code', 'claude-web', 'cursor', 'other']),
	workspaceId: z.string(),
})

// Extract schema accepts EITHER inline content OR R2 key (mutually exclusive)
const extractSchema = extractBaseSchema.and(
	z.union([
		z.object({ content: z.string(), r2Key: z.undefined().optional() }),
		z.object({ r2Key: z.string(), content: z.undefined().optional() }),
	])
)

// Get presigned URL for uploading large conversations to R2
app.post('/upload-url', zValidator('json', uploadUrlSchema), async (c) => {
	const { filename, workspaceId } = c.req.valid('json')
	const orgId = c.get('orgId')
	const timestamp = Date.now()
	const key = `conversations/${orgId}/${workspaceId}/${filename}-${timestamp}.jsonl`

	const s3 = new S3Client({
		region: 'auto',
		endpoint: `https://${c.env.CF_ACCOUNT_ID}.r2.cloudflarestorage.com`,
		credentials: {
			accessKeyId: c.env.R2_ACCESS_KEY_ID,
			secretAccessKey: c.env.R2_SECRET_ACCESS_KEY,
		},
	})

	const uploadUrl = await getSignedUrl(
		s3,
		new PutObjectCommand({
			Bucket: c.env.R2_BUCKET_NAME,
			Key: key,
		}),
		{ expiresIn: 300 }
	)

	return c.json({ uploadUrl, r2Key: key })
})

// Trigger extraction workflow (supports inline content or R2 key for large files)
app.post(
	'/conversations/extract',
	zValidator('json', extractSchema),
	async (c) => {
		const body = c.req.valid('json')
		const { sourcePath, source, workspaceId } = body
		const orgId = c.get('orgId')

		// Determine content vs r2Key mode
		const content = 'content' in body && body.content ? body.content : null
		const r2Key = 'r2Key' in body && body.r2Key ? body.r2Key : null

		const instance = await c.env.EXTRACT_WORKFLOW.create({
			params: { orgId, workspaceId, content, r2Key, sourcePath, source },
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
