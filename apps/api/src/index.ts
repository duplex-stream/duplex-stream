import { Hono } from 'hono'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from './context'

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
