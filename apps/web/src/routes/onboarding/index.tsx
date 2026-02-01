import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getAuth } from '@workos/authkit-tanstack-react-start'
import { useState } from 'react'
import { Button } from '~/components/ui/button'
import { createApiClient } from '~/lib/api'
import '~/styles.css'

const createWorkspace = createServerFn({ method: 'POST' })
	.inputValidator((data: { name: string }) => data)
	.handler(async ({ data }) => {
		const auth = await getAuth()
		if (!auth.user) {
			throw new Error('Not authenticated')
		}
		const api = createApiClient(auth.accessToken)
		return await api.createWorkspace(data.name)
	})

export const Route = createFileRoute('/onboarding/')({
	component: OnboardingPage,
})

function OnboardingPage() {
	const navigate = useNavigate()
	const [workspaceName, setWorkspaceName] = useState('')
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault()
		if (!workspaceName.trim()) return

		setIsSubmitting(true)
		setError(null)

		try {
			await createWorkspace({ data: { name: workspaceName } })
			navigate({ to: '/dashboard' })
		} catch (err) {
			setError(err instanceof Error ? err.message : 'Failed to create workspace')
			setIsSubmitting(false)
		}
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center px-4">
			<div className="w-full max-w-md">
				<div className="text-center">
					<h1 className="text-3xl font-bold">Welcome to Duplex Stream</h1>
					<p className="mt-2 text-muted-foreground">
						Let&apos;s set up your workspace to start capturing decisions.
					</p>
				</div>

				<form onSubmit={handleSubmit} className="mt-8 space-y-6">
					<div>
						<label
							htmlFor="workspace-name"
							className="block text-sm font-medium"
						>
							Workspace Name
						</label>
						<input
							id="workspace-name"
							type="text"
							value={workspaceName}
							onChange={(e) => setWorkspaceName(e.target.value)}
							placeholder="My Workspace"
							className="mt-2 h-10 w-full rounded-md border border-input bg-background px-4 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							required
						/>
						<p className="mt-2 text-sm text-muted-foreground">
							This is your personal workspace where your decisions will be
							stored.
						</p>
					</div>

					{error && (
						<p className="text-sm text-red-500">{error}</p>
					)}

					<Button type="submit" className="w-full" disabled={isSubmitting}>
						{isSubmitting ? 'Creating...' : 'Create Workspace'}
					</Button>
				</form>

				<div className="mt-6 text-center">
					<Link
						to="/dashboard"
						className="text-sm text-muted-foreground hover:text-foreground"
					>
						Skip for now
					</Link>
				</div>
			</div>
		</div>
	)
}
