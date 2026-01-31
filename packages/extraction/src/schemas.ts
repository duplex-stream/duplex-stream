import { z } from 'zod'

// Phase 1: Decision identification
export const DecisionAppearanceSchema = z.object({
	messageStart: z.number(),
	messageEnd: z.number(),
	type: z.enum(['introduced', 'elaborated', 'modified', 'reaffirmed']),
})

export const DecisionCandidateSchema = z.object({
	tempId: z.string(),
	title: z.string(),
	appearances: z.array(DecisionAppearanceSchema),
	confidence: z.number(), // 0-1 range, validated in prompts
})

export const IdentificationResponseSchema = z.object({
	decisions: z.array(DecisionCandidateSchema),
})

// Phase 2: Decision extraction
export const AlternativeSchema = z.object({
	description: z.string(),
	whyRejected: z.string(),
})

export const ExtractedDecisionSchema = z.object({
	summary: z.string(),
	reasoning: z.string(),
	alternativesConsidered: z.array(AlternativeSchema),
	status: z.enum(['active', 'superseded', 'tentative']),
	dependsOn: z.array(z.string()),
	confidence: z.number(), // 0-1 range, validated in prompts
})

// Inferred types
export type DecisionAppearance = z.infer<typeof DecisionAppearanceSchema>
export type DecisionCandidate = z.infer<typeof DecisionCandidateSchema>
export type IdentificationResponse = z.infer<typeof IdentificationResponseSchema>
export type Alternative = z.infer<typeof AlternativeSchema>
export type ExtractedDecision = z.infer<typeof ExtractedDecisionSchema>
