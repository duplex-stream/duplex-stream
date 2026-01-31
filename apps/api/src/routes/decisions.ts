import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import { eq, and, or, like, desc } from 'drizzle-orm'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from '../context'

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
