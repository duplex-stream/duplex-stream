import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Button } from '~/components/ui/button'
import '~/styles.css'

export const Route = createFileRoute('/onboarding/')({
	component: OnboardingPage,
})

function OnboardingPage() {
	const navigate = useNavigate()
	const [workspaceName, setWorkspaceName] = useState('')
	const [isSubmitting, setIsSubmitting] = useState(false)

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault()
		if (!workspaceName.trim()) return

		setIsSubmitting(true)
		// TODO: Create workspace via API
		// For now, just navigate to dashboard
		navigate({ to: '/dashboard' })
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
