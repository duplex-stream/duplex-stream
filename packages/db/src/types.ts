import type { InferSelectModel, InferInsertModel } from 'drizzle-orm'
import * as schema from './schema/index.js'

// Base types from schema
export type Conversation = InferSelectModel<typeof schema.conversations>
export type NewConversation = InferInsertModel<typeof schema.conversations>

export type Message = InferSelectModel<typeof schema.messages>
export type NewMessage = InferInsertModel<typeof schema.messages>

export type Decision = InferSelectModel<typeof schema.decisions>
export type NewDecision = InferInsertModel<typeof schema.decisions>

export type DecisionAppearance = InferSelectModel<typeof schema.decisionAppearances>
export type NewDecisionAppearance = InferInsertModel<typeof schema.decisionAppearances>

export type Alternative = InferSelectModel<typeof schema.alternatives>
export type NewAlternative = InferInsertModel<typeof schema.alternatives>

export type DecisionDependency = InferSelectModel<typeof schema.decisionDependencies>
export type NewDecisionDependency = InferInsertModel<typeof schema.decisionDependencies>

// Composite types for queries
export type DecisionWithRelations = Decision & {
	appearances: DecisionAppearance[]
	alternatives: Alternative[]
	dependencies: DecisionDependency[]
	conversation?: Conversation
}

export type ConversationWithRelations = Conversation & {
	messages: Message[]
	decisions: Decision[]
}
