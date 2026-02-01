interface ClaudeCodeEvent {
	type: string
	message?: {
		role: string
		content: string | Array<{ type: string; text?: string; thinking?: string }>
	}
	timestamp?: string
	sessionId?: string
}

export interface ParsedMessage {
	index: number
	role: 'user' | 'assistant' | 'system'
	content: string
	thinking?: string
	timestamp?: Date
}

export interface ParsedConversation {
	sessionId: string
	projectPath: string
	messages: ParsedMessage[]
	createdAt: Date
}

export interface ParseOptions {
	sourcePath?: string
}

export function parseClaudeCodeSession(
	content: string,
	options: ParseOptions = {}
): ParsedConversation {
	const lines = content.trim().split('\n')

	const messages: ParsedMessage[] = []
	let sessionId = ''
	let projectPath = ''
	let earliestTimestamp: Date | null = null

	for (const line of lines) {
		try {
			const event: ClaudeCodeEvent = JSON.parse(line)

			if (event.sessionId && !sessionId) {
				sessionId = event.sessionId
			}

			if (event.timestamp) {
				const ts = new Date(event.timestamp)
				if (!earliestTimestamp || ts < earliestTimestamp) {
					earliestTimestamp = ts
				}
			}

			if (event.type === 'user' && event.message) {
				messages.push({
					index: messages.length,
					role: 'user',
					content:
						typeof event.message.content === 'string'
							? event.message.content
							: '',
					timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
				})
			}

			if (event.type === 'assistant' && event.message) {
				const contentArray = Array.isArray(event.message.content)
					? event.message.content
					: []

				const textContent = contentArray
					.filter((block) => block.type === 'text')
					.map((block) => block.text || '')
					.join('\n')

				const thinkingContent = contentArray
					.filter((block) => block.type === 'thinking')
					.map((block) => block.thinking || '')
					.join('\n')

				if (textContent || thinkingContent) {
					messages.push({
						index: messages.length,
						role: 'assistant',
						content: textContent,
						thinking: thinkingContent || undefined,
						timestamp: event.timestamp ? new Date(event.timestamp) : undefined,
					})
				}
			}

			// Skip system messages entirely - they contain internal Claude Code
			// instructions (compaction, formatting, tool definitions) not project decisions
			// Actual decisions are captured in user/assistant message exchanges
			if (event.type === 'system') {
				continue
			}
		} catch {
			continue
		}
	}

	if (options.sourcePath) {
		const match = options.sourcePath.match(/projects\/([^/]+)\//)
		if (match) {
			projectPath = match[1].replace(/-/g, '/')
			if (projectPath.startsWith('/')) {
				projectPath = projectPath.slice(1)
			}
		}
	}

	return {
		sessionId,
		projectPath,
		messages,
		createdAt: earliestTimestamp || new Date(),
	}
}
