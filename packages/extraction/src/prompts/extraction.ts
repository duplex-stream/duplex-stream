import type { DecisionCandidate } from '../schemas'

export function buildExtractionPrompt(
	candidate: DecisionCandidate,
	contextMessages: string,
	otherCandidates: DecisionCandidate[]
): string {
	const otherDecisionsList = otherCandidates
		.filter((c) => c.tempId !== candidate.tempId)
		.map((c) => `- [${c.tempId}] ${c.title}`)
		.join('\n')

	const appearancesList = candidate.appearances
		.map((a) => `- Messages ${a.messageStart}-${a.messageEnd} (${a.type})`)
		.join('\n')

	return `You are extracting detailed information about a specific decision from a conversation.

DECISION: "${candidate.title}"
APPEARS AT:
${appearancesList}

RELEVANT CONTEXT (messages around each appearance):
${contextMessages}

OTHER DECISIONS IN THIS CONVERSATION (for dependency matching):
${otherDecisionsList || '(none)'}

Extract:
- summary: One paragraph explaining what was decided
- reasoning: The actual reasoning WHY this decision was made. Not just "they discussed it" but the specific logic, constraints, and considerations that led to this choice.
- alternativesConsidered: Array of alternatives that were discussed but not chosen
  - description: What the alternative was
  - whyRejected: Why it wasn't chosen
- status: One of:
  - "active": This is the current decision
  - "superseded": This was later replaced by another decision
  - "tentative": This was proposed but not firmly committed to
- dependsOn: Array of tempIds from other decisions that this decision depends on or builds upon. Only include clear dependencies, not vague relationships.
- confidence: 0-1 how confident you are in the accuracy of this extraction

EXAMPLE OUTPUT:
{
  "summary": "Use file watching on AI tool history directories rather than MCP for capture. The system will monitor ~/.claude/projects/ and similar directories for conversation files, parse them, and run extraction.",
  "reasoning": "MCP puts the AI tool in control of what gets sent to duplex stream. But the AI doesn't know what context will matter later - it would filter based on what seems relevant now. This creates the same problem as vibe coding: reasoning gets lost because the source decides what's important. By watching files instead, duplex stream controls capture and can extract everything, deciding importance through extraction rather than filtering at the source.",
  "alternativesConsidered": [
    {
      "description": "MCP server for both reading and writing - AI tools would push events to duplex stream",
      "whyRejected": "AI tool controls what gets captured, same filtering problem as vibe coding"
    },
    {
      "description": "Per-tool adapters using tool-specific APIs",
      "whyRejected": "Still relies on the tool to decide what to expose"
    }
  ],
  "status": "active",
  "dependsOn": ["decision_1"],
  "confidence": 0.95
}`
}
