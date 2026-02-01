import { Hono } from 'hono'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { HonoEnv } from './context'
import { authMiddleware } from './middleware/auth'
import decisionsRoutes from './routes/decisions'
import extractionRoutes from './routes/extraction'
import workspacesRoutes from './routes/workspaces'

const app = new Hono<HonoEnv>()

// Middleware: Database setup (all routes)
app.use('*', async (c, next) => {
	const db = drizzle(c.env.DB, { schema })
	c.set('db', db)
	await next()
})

// Health check (public, no auth required)
app.get('/', (c) => {
	return c.json({ status: 'ok', service: 'duplex-api' })
})

// Protected routes - require authentication
app.use('/decisions/*', authMiddleware())
app.use('/extraction/*', authMiddleware())
app.use('/workspaces/*', authMiddleware())

// Routes
app.route('/decisions', decisionsRoutes)
app.route('/extraction', extractionRoutes)
app.route('/workspaces', workspacesRoutes)

export default app

export { ExtractConversationWorkflow } from './workflows/extract-conversation'
