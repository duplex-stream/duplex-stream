import type { ParsedMessage } from './parsers/claude-code'

export function buildTranscript(messages: ParsedMessage[]): string {
	return messages
		.map((m) => {
			let text = `[${m.index}] ${m.role.toUpperCase()}: ${m.content}`
			if (m.thinking) {
				text += `\n[${m.index}] THINKING: ${m.thinking}`
			}
			return text
		})
		.join('\n\n')
}

interface Appearance {
	messageStart: number
	messageEnd: number
	type: string
}

interface ContextWindowOptions {
	buffer?: number
}

export function buildContextWindow(
	messages: ParsedMessage[],
	appearances: Appearance[],
	options: ContextWindowOptions = {}
): string {
	const { buffer = 2 } = options

	const indices = new Set<number>()

	for (const appearance of appearances) {
		const start = Math.max(0, appearance.messageStart - buffer)
		const end = Math.min(messages.length - 1, appearance.messageEnd + buffer)

		for (let i = start; i <= end; i++) {
			indices.add(i)
		}
	}

	const sortedIndices = Array.from(indices).sort((a, b) => a - b)

	return sortedIndices
		.map((i) => {
			const m = messages[i]
			let text = `[${m.index}] ${m.role.toUpperCase()}: ${m.content}`
			if (m.thinking) {
				text += `\n[${m.index}] THINKING: ${m.thinking}`
			}
			return text
		})
		.join('\n\n')
}
