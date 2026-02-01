import { createFileRoute, Link } from '@tanstack/react-router'
import { Download, Check } from 'lucide-react'
import { Button } from '~/components/ui/button'
import '~/styles.css'

export const Route = createFileRoute('/onboarding/desktop')({
	component: DesktopOnboardingPage,
})

function DesktopOnboardingPage() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center px-4">
			<div className="w-full max-w-lg text-center">
				<div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary">
					<Download className="h-8 w-8 text-primary-foreground" />
				</div>

				<h1 className="mt-6 text-3xl font-bold">Install the Desktop App</h1>
				<p className="mt-2 text-muted-foreground">
					The desktop app automatically captures decisions from your AI
					conversations.
				</p>

				<div className="mt-8 rounded-lg border p-6 text-left">
					<h2 className="font-semibold">How it works</h2>
					<ul className="mt-4 space-y-3">
						<li className="flex items-start gap-3">
							<Check className="mt-0.5 h-5 w-5 text-green-600" />
							<span className="text-sm">
								Runs in the background and monitors clipboard activity
							</span>
						</li>
						<li className="flex items-start gap-3">
							<Check className="mt-0.5 h-5 w-5 text-green-600" />
							<span className="text-sm">
								Detects when you copy AI conversation text
							</span>
						</li>
						<li className="flex items-start gap-3">
							<Check className="mt-0.5 h-5 w-5 text-green-600" />
							<span className="text-sm">
								Automatically extracts and saves decisions
							</span>
						</li>
						<li className="flex items-start gap-3">
							<Check className="mt-0.5 h-5 w-5 text-green-600" />
							<span className="text-sm">
								Syncs decisions to your Duplex Stream account
							</span>
						</li>
					</ul>
				</div>

				<div className="mt-8 space-y-4">
					<Button className="w-full" size="lg">
						<Download className="mr-2 h-4 w-4" />
						Download for macOS
					</Button>
					<p className="text-sm text-muted-foreground">
						Requires macOS 12.0 or later
					</p>
				</div>

				<div className="mt-8 border-t pt-8">
					<Link
						to="/dashboard"
						className="text-sm text-muted-foreground hover:text-foreground"
					>
						Skip for now and continue to dashboard
					</Link>
				</div>
			</div>
		</div>
	)
}
