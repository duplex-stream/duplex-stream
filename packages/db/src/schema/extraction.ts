import { sqliteTable, text, integer, real } from 'drizzle-orm/sqlite-core'
import { relations } from 'drizzle-orm'

// Conversations table
export const conversations = sqliteTable('conversations', {
	id: text('id').primaryKey(),
	orgId: text('org_id').notNull(),
	workspaceId: text('workspace_id').notNull(),
	source: text('source', {
		enum: ['claude-code', 'claude-web', 'cursor', 'other'],
	}).notNull(),
	sourcePath: text('source_path').notNull(),
	projectPath: text('project_path').notNull(),
	sessionId: text('session_id'),
	messageCount: integer('message_count').notNull(),
	createdAt: text('created_at').notNull(),
	extractedAt: text('extracted_at'),
})

// Messages table
export const messages = sqliteTable('messages', {
	id: text('id').primaryKey(),
	conversationId: text('conversation_id')
		.notNull()
		.references(() => conversations.id, { onDelete: 'cascade' }),
	index: integer('index').notNull(),
	role: text('role', { enum: ['user', 'assistant', 'system'] }).notNull(),
	content: text('content').notNull(),
	thinking: text('thinking'),
	timestamp: text('timestamp'),
})

// Decisions table
export const decisions = sqliteTable('decisions', {
	id: text('id').primaryKey(),
	conversationId: text('conversation_id')
		.notNull()
		.references(() => conversations.id, { onDelete: 'cascade' }),
	orgId: text('org_id').notNull(),
	workspaceId: text('workspace_id').notNull(),
	title: text('title').notNull(),
	summary: text('summary').notNull(),
	reasoning: text('reasoning').notNull(),
	status: text('status', {
		enum: ['active', 'superseded', 'tentative'],
	}).notNull(),
	confidence: real('confidence').notNull(),
	extractedAt: text('extracted_at').notNull(),
})

// Decision appearances (non-contiguous locations)
export const decisionAppearances = sqliteTable('decision_appearances', {
	id: text('id').primaryKey(),
	decisionId: text('decision_id')
		.notNull()
		.references(() => decisions.id, { onDelete: 'cascade' }),
	messageStart: integer('message_start').notNull(),
	messageEnd: integer('message_end').notNull(),
	type: text('type', {
		enum: ['introduced', 'elaborated', 'modified', 'reaffirmed'],
	}).notNull(),
	context: text('context'),
})

// Alternatives considered
export const alternatives = sqliteTable('alternatives', {
	id: text('id').primaryKey(),
	decisionId: text('decision_id')
		.notNull()
		.references(() => decisions.id, { onDelete: 'cascade' }),
	description: text('description').notNull(),
	whyRejected: text('why_rejected').notNull(),
})

// Decision dependencies (graph edges)
export const decisionDependencies = sqliteTable('decision_dependencies', {
	id: text('id').primaryKey(),
	fromDecisionId: text('from_decision_id')
		.notNull()
		.references(() => decisions.id, { onDelete: 'cascade' }),
	toDecisionRef: text('to_decision_ref').notNull(),
})

// Relations
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

export const decisionAppearancesRelations = relations(
	decisionAppearances,
	({ one }) => ({
		decision: one(decisions, {
			fields: [decisionAppearances.decisionId],
			references: [decisions.id],
		}),
	})
)

export const alternativesRelations = relations(alternatives, ({ one }) => ({
	decision: one(decisions, {
		fields: [alternatives.decisionId],
		references: [decisions.id],
	}),
}))

export const decisionDependenciesRelations = relations(
	decisionDependencies,
	({ one }) => ({
		fromDecision: one(decisions, {
			fields: [decisionDependencies.fromDecisionId],
			references: [decisions.id],
		}),
	})
)
