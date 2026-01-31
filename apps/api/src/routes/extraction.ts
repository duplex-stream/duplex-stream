import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import type { HonoEnv } from '../context'

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
