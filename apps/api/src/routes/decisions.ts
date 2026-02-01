import { Hono } from 'hono'
import { zValidator } from '@hono/zod-validator'
import { z } from 'zod'
import { eq, and, or, like, desc, count } from 'drizzle-orm'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from '../context'

const app = new Hono<HonoEnv>()

// List/search decisions
app.get(
	'/',
	zValidator(
		'query',
		z.object({
			q: z.string().optional(),
			search: z.string().optional(),
			workspaceId: z.string().optional(),
			page: z.coerce.number().optional().default(1),
			pageSize: z.coerce.number().optional().default(20),
			limit: z.coerce.number().optional(), // deprecated, use pageSize
		})
	),
	async (c) => {
		const { q, search, workspaceId, page, pageSize, limit } = c.req.valid('query')
		const orgId = c.get('orgId')
		const db = c.get('db')

		const searchTerm = q || search
		const effectiveLimit = pageSize || limit || 20
		const offset = (page - 1) * effectiveLimit

		// Build conditions
		const conditions = [eq(schema.decisions.orgId, orgId)]

		if (searchTerm) {
			conditions.push(
				or(
					like(schema.decisions.title, `%${searchTerm}%`),
					like(schema.decisions.summary, `%${searchTerm}%`),
					like(schema.decisions.reasoning, `%${searchTerm}%`)
				)!
			)
		}

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
			limit: effectiveLimit,
			offset,
		})

		// Count total for pagination
		const totalCount = await db
			.select({ count: count() })
			.from(schema.decisions)
			.where(and(...conditions))

		return c.json({
			data: decisions,
			total: totalCount[0].count,
			page,
			pageSize: effectiveLimit,
		})
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

	return c.json(decision)
})

export default app
