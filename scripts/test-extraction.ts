#!/usr/bin/env bun

import { readFile } from 'fs/promises'
import { parseClaudeCodeSession } from '../packages/extraction/src/parsers/claude-code'
import { buildTranscript } from '../packages/extraction/src/transcript'

async function main() {
	const sessionPath = process.argv[2]

	if (!sessionPath) {
		console.error('Usage: bun scripts/test-extraction.ts <session-file.jsonl>')
		process.exit(1)
	}

	console.log(`Reading: ${sessionPath}`)
	const content = await readFile(sessionPath, 'utf-8')

	console.log('Parsing...')
	const conversation = parseClaudeCodeSession(content, {
		sourcePath: sessionPath,
	})

	console.log(`Session ID: ${conversation.sessionId}`)
	console.log(`Project: ${conversation.projectPath}`)
	console.log(`Messages: ${conversation.messages.length}`)
	console.log(`Created: ${conversation.createdAt.toISOString()}`)

	console.log('\n--- Message Summary ---')
	const userCount = conversation.messages.filter(
		(m) => m.role === 'user'
	).length
	const assistantCount = conversation.messages.filter(
		(m) => m.role === 'assistant'
	).length
	const systemCount = conversation.messages.filter(
		(m) => m.role === 'system'
	).length
	console.log(
		`User: ${userCount}, Assistant: ${assistantCount}, System: ${systemCount}`
	)

	console.log('\n--- Transcript Preview (first 2000 chars) ---')
	const transcript = buildTranscript(conversation.messages)
	console.log(transcript.slice(0, 2000))
	console.log('...')

	console.log(`\nTotal transcript length: ${transcript.length} chars`)
}

main().catch(console.error)
