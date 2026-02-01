import { createFileRoute, Link } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { Search } from 'lucide-react'
import { useState } from 'react'
import { Button } from '~/components/ui/button'

interface Decision {
	id: string
	title: string
	summary: string
	status: 'active' | 'archived'
	createdAt: string
	updatedAt: string
}

interface DecisionsResponse {
	decisions: Decision[]
	total: number
	page: number
	pageSize: number
}

const fetchDecisions = createServerFn({ method: 'GET' })
	.inputValidator(
		(data: { page?: number; pageSize?: number; search?: string }) => data,
	)
	.handler(async ({ data }) => {
		// TODO: Call the actual API with auth token
		// For now, return mock data
		const mockDecisions: Decision[] = [
			{
				id: '1',
				title: 'Use TypeScript for all new projects',
				summary:
					'Agreed to standardize on TypeScript for improved type safety and developer experience.',
				status: 'active',
				createdAt: '2025-01-15T10:00:00Z',
				updatedAt: '2025-01-15T10:00:00Z',
			},
			{
				id: '2',
				title: 'Implement dark mode with CSS variables',
				summary:
					'Decided to use CSS custom properties for theming to support dark mode.',
				status: 'active',
				createdAt: '2025-01-14T15:30:00Z',
				updatedAt: '2025-01-14T15:30:00Z',
			},
		]

		const filtered = data.search
			? mockDecisions.filter(
					(d) =>
						d.title.toLowerCase().includes(data.search!.toLowerCase()) ||
						d.summary.toLowerCase().includes(data.search!.toLowerCase()),
				)
			: mockDecisions

		return {
			decisions: filtered,
			total: filtered.length,
			page: data.page || 1,
			pageSize: data.pageSize || 10,
		} as DecisionsResponse
	})

export const Route = createFileRoute('/_authed/dashboard')({
	validateSearch: (search: Record<string, unknown>) => ({
		page: Number(search.page) || 1,
		search: (search.search as string) || undefined,
	}),
	loaderDeps: ({ search }) => ({ search }),
	loader: async ({ deps }) => {
		const response = await fetchDecisions({
			data: {
				page: deps.search.page,
				search: deps.search.search,
			},
		})
		return response
	},
	component: DashboardPage,
})

function DashboardPage() {
	const data = Route.useLoaderData()
	const search = Route.useSearch()
	const [searchQuery, setSearchQuery] = useState(search.search || '')

	const handleSearch = (e: React.FormEvent) => {
		e.preventDefault()
		// Navigate with search query
		window.location.href = `/dashboard?search=${encodeURIComponent(searchQuery)}`
	}

	return (
		<div className="container mx-auto px-4 py-8">
			<div className="mb-8 flex items-center justify-between">
				<h1 className="text-3xl font-bold">Your Decisions</h1>
			</div>

			<form onSubmit={handleSearch} className="mb-6">
				<div className="relative max-w-md">
					<Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
					<input
						type="text"
						placeholder="Search decisions..."
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						className="h-10 w-full rounded-md border border-input bg-background pl-10 pr-4 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
					/>
				</div>
			</form>

			{data.decisions.length === 0 ? (
				<EmptyState search={search.search} />
			) : (
				<div className="space-y-4">
					{data.decisions.map((decision) => (
						<DecisionCard key={decision.id} decision={decision} />
					))}
				</div>
			)}

			{data.total > data.pageSize && (
				<Pagination
					page={data.page}
					totalPages={Math.ceil(data.total / data.pageSize)}
					search={search.search}
				/>
			)}
		</div>
	)
}

function DecisionCard({ decision }: { decision: Decision }) {
	return (
		<Link
			to="/decisions/$id"
			params={{ id: decision.id }}
			className="block rounded-lg border bg-card p-6 shadow-sm transition-colors hover:bg-accent/50"
		>
			<div className="flex items-start justify-between">
				<div>
					<h3 className="font-semibold">{decision.title}</h3>
					<p className="mt-1 text-sm text-muted-foreground">
						{decision.summary}
					</p>
				</div>
				<span
					className={`rounded-full px-2 py-1 text-xs font-medium ${
						decision.status === 'active'
							? 'bg-green-100 text-green-700'
							: 'bg-gray-100 text-gray-700'
					}`}
				>
					{decision.status}
				</span>
			</div>
			<div className="mt-4 text-xs text-muted-foreground">
				Updated {new Date(decision.updatedAt).toLocaleDateString()}
			</div>
		</Link>
	)
}

function EmptyState({ search }: { search?: string }) {
	return (
		<div className="flex flex-col items-center justify-center py-16 text-center">
			<div className="rounded-full bg-muted p-4">
				<Search className="h-8 w-8 text-muted-foreground" />
			</div>
			<h3 className="mt-4 text-lg font-semibold">No decisions found</h3>
			<p className="mt-2 text-sm text-muted-foreground">
				{search
					? `No decisions match "${search}". Try a different search term.`
					: 'Start a conversation with an AI assistant and Duplex Stream will capture your decisions.'}
			</p>
		</div>
	)
}

function Pagination({
	page,
	totalPages,
	search,
}: {
	page: number
	totalPages: number
	search?: string
}) {
	const buildUrl = (pageNum: number) => {
		const params = new URLSearchParams()
		params.set('page', pageNum.toString())
		if (search) params.set('search', search)
		return `/dashboard?${params.toString()}`
	}

	return (
		<div className="mt-8 flex items-center justify-center gap-2">
			<Button
				variant="outline"
				size="sm"
				disabled={page <= 1}
				asChild={page > 1}
			>
				{page > 1 ? <Link to={buildUrl(page - 1)}>Previous</Link> : 'Previous'}
			</Button>
			<span className="text-sm text-muted-foreground">
				Page {page} of {totalPages}
			</span>
			<Button
				variant="outline"
				size="sm"
				disabled={page >= totalPages}
				asChild={page < totalPages}
			>
				{page < totalPages ? <Link to={buildUrl(page + 1)}>Next</Link> : 'Next'}
			</Button>
		</div>
	)
}
