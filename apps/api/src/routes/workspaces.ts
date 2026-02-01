import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import type { HonoEnv } from '../context'

const app = new Hono<HonoEnv>()

// Create workspace
app.post(
	'/',
	zValidator(
		'json',
		z.object({
			name: z.string().min(1),
		})
	),
	async (c) => {
		const { name } = c.req.valid('json')
		const _userId = c.get('userId')

		// Generate a simple ID (in production, use UUID or database-generated ID)
		const id = crypto.randomUUID()
		const createdAt = new Date().toISOString()

		// TODO: Persist workspace to database when schema is available

		return c.json({
			id,
			name,
			createdAt,
		}, 201)
	}
)

// Get current workspace
app.get('/current', async (c) => {
	const _userId = c.get('userId')

	// TODO: Fetch current workspace from database when schema is available

	return c.json({
		id: 'default',
		name: 'Default Workspace',
	})
})

export default app
