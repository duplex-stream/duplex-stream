export function buildIdentificationPrompt(transcript: string): string {
	return `You are analyzing a conversation to identify decisions that were made.

A decision is:
- An explicit choice between alternatives
- A commitment to an approach, architecture, or implementation
- A constraint or principle that guides other choices

For each decision, provide:
- tempId: Temporary identifier (e.g., "decision_1", "decision_2")
- title: Short descriptive name (5-10 words)
- appearances: Where this decision appears in the conversation
  - messageStart: First message index of this appearance
  - messageEnd: Last message index of this appearance
  - type: One of "introduced", "elaborated", "modified", "reaffirmed"
- confidence: 0-1 how certain you are this is a real decision (not just discussion)

IMPORTANT: Decisions often evolve across a conversation. If a decision is introduced in messages 5-8, then modified in messages 23-25, list BOTH appearances with appropriate types.

EXAMPLE OUTPUT:
{
  "decisions": [
    {
      "tempId": "decision_1",
      "title": "Use file watching for conversation capture",
      "appearances": [
        {"messageStart": 12, "messageEnd": 14, "type": "introduced"},
        {"messageStart": 28, "messageEnd": 30, "type": "elaborated"}
      ],
      "confidence": 0.95
    },
    {
      "tempId": "decision_2",
      "title": "Two-phase extraction pipeline",
      "appearances": [
        {"messageStart": 18, "messageEnd": 22, "type": "introduced"}
      ],
      "confidence": 0.9
    }
  ]
}

CONVERSATION:
${transcript}`
}
