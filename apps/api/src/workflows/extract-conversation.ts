import {
	WorkflowEntrypoint,
	WorkflowStep,
	WorkflowEvent,
} from 'cloudflare:workers'
import { createAnthropic } from '@ai-sdk/anthropic'
import { generateObject } from 'ai'
import { parseClaudeCodeSession } from '@repo/extraction/parsers'
import { buildTranscript, buildContextWindow } from '@repo/extraction'
import {
	buildIdentificationPrompt,
	buildExtractionPrompt,
} from '@repo/extraction/prompts'
import {
	IdentificationResponseSchema,
	ExtractedDecisionSchema,
	type DecisionCandidate,
} from '@repo/extraction'
import { drizzle } from 'drizzle-orm/d1'
import * as schema from '@repo/db/schema'
import type { Env } from '../context'

interface ExtractConversationParams {
	orgId: string
	workspaceId: string
	content: string
	sourcePath: string
	source: 'claude-code' | 'claude-web' | 'cursor' | 'other'
}

interface ExtractConversationResult {
	conversationId: string
	decisionCount: number
}

export class ExtractConversationWorkflow extends WorkflowEntrypoint<
	Env,
	ExtractConversationParams,
	ExtractConversationResult
> {
	async run(
		event: WorkflowEvent<ExtractConversationParams>,
		step: WorkflowStep
	): Promise<ExtractConversationResult> {
		const { orgId, workspaceId, content, sourcePath, source } = event.payload

		// Create AI client - using direct Anthropic API for now
		// TODO: Re-enable AI Gateway once auth is working
		const client = createAnthropic({
			apiKey: this.env.ANTHROPIC_API_KEY!,
		})

		// Step 1: Parse content
		const conversation = await step.do('parse-content', async () => {
			switch (source) {
				case 'claude-code':
					return parseClaudeCodeSession(content, { sourcePath })
				default:
					throw new Error(`Parser not implemented for source: ${source}`)
			}
		})

		// Step 2: Build transcript
		const transcript = await step.do('build-transcript', async () => {
			return buildTranscript(conversation.messages)
		})

		// Step 3: Identify decisions (Phase 1)
		const candidates = await step.do('identify-decisions', async () => {
			const { object } = await generateObject({
				model: client('claude-sonnet-4-5-20250929'),
				schema: IdentificationResponseSchema,
				prompt: buildIdentificationPrompt(transcript),
			})
			return object.decisions
		})

		// Step 4: Extract each decision (Phase 2)
		const extracted: Array<
			DecisionCandidate & {
				summary: string
				reasoning: string
				status: string
				dependsOn: string[]
				alternativesConsidered: Array<{
					description: string
					whyRejected: string
				}>
			}
		> = []

		for (const [i, candidate] of candidates.entries()) {
			const decision = await step.do(`extract-decision-${i}`, async () => {
				const contextMessages = buildContextWindow(
					conversation.messages,
					candidate.appearances,
					{ buffer: 2 }
				)

				const { object } = await generateObject({
					model: client('claude-sonnet-4-5-20250929'),
					schema: ExtractedDecisionSchema,
					prompt: buildExtractionPrompt(candidate, contextMessages, candidates),
				})

				return {
					...candidate,
					...object,
				}
			})

			extracted.push(decision)
		}

		// Step 5: Store results
		const result = await step.do('store-results', async () => {
			const db = drizzle(this.env.DB, { schema })
			const conversationId = crypto.randomUUID()
			const now = new Date().toISOString()

			// Build ID mapping
			const idMap = new Map<string, string>()
			for (const d of extracted) {
				idMap.set(d.tempId, crypto.randomUUID())
			}

			// Insert conversation
			await db.insert(schema.conversations).values({
				id: conversationId,
				orgId,
				workspaceId,
				source,
				sourcePath,
				projectPath: conversation.projectPath,
				sessionId: conversation.sessionId,
				messageCount: conversation.messages.length,
				createdAt: conversation.createdAt.toISOString(),
				extractedAt: now,
			})

			// Insert messages
			for (const msg of conversation.messages) {
				await db.insert(schema.messages).values({
					id: crypto.randomUUID(),
					conversationId,
					index: msg.index,
					role: msg.role,
					content: msg.content,
					thinking: msg.thinking,
					timestamp: msg.timestamp?.toISOString(),
				})
			}

			// Insert decisions and related data
			for (const d of extracted) {
				const decisionId = idMap.get(d.tempId)!

				await db.insert(schema.decisions).values({
					id: decisionId,
					conversationId,
					orgId,
					workspaceId,
					title: d.title,
					summary: d.summary,
					reasoning: d.reasoning,
					status: d.status as 'active' | 'superseded' | 'tentative',
					confidence: d.confidence,
					extractedAt: now,
				})

				// Insert appearances
				for (const app of d.appearances) {
					await db.insert(schema.decisionAppearances).values({
						id: crypto.randomUUID(),
						decisionId,
						messageStart: app.messageStart,
						messageEnd: app.messageEnd,
						type: app.type,
					})
				}

				// Insert alternatives
				for (const alt of d.alternativesConsidered) {
					await db.insert(schema.alternatives).values({
						id: crypto.randomUUID(),
						decisionId,
						description: alt.description,
						whyRejected: alt.whyRejected,
					})
				}

				// Insert dependencies
				for (const depTempId of d.dependsOn) {
					const resolvedId = idMap.get(depTempId)
					await db.insert(schema.decisionDependencies).values({
						id: crypto.randomUUID(),
						fromDecisionId: decisionId,
						toDecisionRef: resolvedId ? `decision:${resolvedId}` : depTempId,
					})
				}
			}

			return { conversationId }
		})

		return {
			conversationId: result.conversationId,
			decisionCount: extracted.length,
		}
	}
}
