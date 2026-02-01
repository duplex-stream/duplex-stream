import { createFileRoute, Link } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getAuth } from '@workos/authkit-tanstack-react-start'
import { ArrowLeft, Calendar, MessageSquare } from 'lucide-react'
import { createApiClient } from '~/lib/api'
import { Button } from '~/components/ui/button'

const fetchDecision = createServerFn({ method: 'GET' })
	.inputValidator((data: { id: string }) => data)
	.handler(async ({ data }) => {
		const auth = await getAuth()
		if (!auth.user) throw new Error('Not authenticated')

		const api = createApiClient(auth.accessToken)
		return api.getDecision(data.id)
	})

export const Route = createFileRoute('/_authed/decisions/$id')({
	loader: async ({ params }) => {
		return await fetchDecision({ data: { id: params.id } })
	},
	component: DecisionDetailPage,
})

function DecisionDetailPage() {
	const decision = Route.useLoaderData()

	return (
		<div className="container mx-auto px-4 py-8">
			<div className="mb-6">
				<Button variant="ghost" size="sm" asChild>
					<Link to="/dashboard">
						<ArrowLeft className="mr-2 h-4 w-4" />
						Back to Dashboard
					</Link>
				</Button>
			</div>

			<article className="mx-auto max-w-3xl">
				<header className="mb-8">
					<div className="mb-4 flex items-center gap-2">
						<span
							className={`rounded-full px-2 py-1 text-xs font-medium ${
								decision.status === 'active'
									? 'bg-green-100 text-green-700'
									: 'bg-gray-100 text-gray-700'
							}`}
						>
							{decision.status}
						</span>
						<span className="text-sm text-muted-foreground">
							from {decision.source?.type ?? 'unknown'}
						</span>
					</div>

					<h1 className="text-3xl font-bold">{decision.title}</h1>

					<p className="mt-4 text-lg text-muted-foreground">
						{decision.summary}
					</p>

					<div className="mt-6 flex items-center gap-6 text-sm text-muted-foreground">
						<div className="flex items-center gap-1">
							<Calendar className="h-4 w-4" />
							<span>
								Created {new Date(decision.createdAt).toLocaleDateString()}
							</span>
						</div>
						<div className="flex items-center gap-1">
							<MessageSquare className="h-4 w-4" />
							<span>
								{decision.source?.type === 'claude' ? 'Claude' : decision.source?.type === 'chatgpt' ? 'ChatGPT' : 'AI Assistant'}
							</span>
						</div>
					</div>
				</header>

				<section className="prose prose-gray max-w-none">
					<h2 className="text-xl font-semibold">Context</h2>
					<div className="mt-4 whitespace-pre-wrap rounded-lg bg-muted/50 p-6">
						{decision.context ?? ''}
					</div>
				</section>

				{(decision.tags?.length ?? 0) > 0 && (
					<section className="mt-8">
						<h2 className="text-xl font-semibold">Tags</h2>
						<div className="mt-4 flex flex-wrap gap-2">
							{decision.tags?.map((tag) => (
								<span
									key={tag}
									className="rounded-full bg-secondary px-3 py-1 text-sm text-secondary-foreground"
								>
									{tag}
								</span>
							))}
						</div>
					</section>
				)}

				<section className="mt-8 flex gap-4">
					<Button variant="outline">Archive Decision</Button>
					<Button variant="outline">Edit</Button>
				</section>
			</article>
		</div>
	)
}
