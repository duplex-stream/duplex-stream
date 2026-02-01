import { createFileRoute, Link } from '@tanstack/react-router'
import '~/styles.css'

export const Route = createFileRoute('/')({
	component: HomePage,
})

function HomePage() {
	return (
		<div className="flex min-h-screen flex-col">
			<header className="border-b">
				<div className="container mx-auto flex h-16 items-center justify-between px-4">
					<Link to="/" className="text-xl font-bold">
						Duplex Stream
					</Link>
					<nav className="flex items-center gap-4">
						<Link
							to="/auth/login"
							className="text-sm font-medium text-muted-foreground hover:text-foreground"
						>
							Sign In
						</Link>
						<Link
							to="/auth/login"
							className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
						>
							Get Started
						</Link>
					</nav>
				</div>
			</header>

			<main className="flex flex-1 flex-col items-center justify-center px-4">
				<div className="mx-auto max-w-3xl text-center">
					<h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
						Turn AI Conversations into{' '}
						<span className="text-primary">Lasting Decisions</span>
					</h1>
					<p className="mt-6 text-lg leading-8 text-muted-foreground">
						Duplex Stream captures and organizes the decisions you make with AI
						assistants, so you never lose track of what you agreed on.
					</p>
					<div className="mt-10 flex items-center justify-center gap-x-6">
						<Link
							to="/auth/login"
							className="rounded-md bg-primary px-6 py-3 text-sm font-semibold text-primary-foreground shadow-sm hover:bg-primary/90"
						>
							Start Free
						</Link>
						<a
							href="#how-it-works"
							className="text-sm font-semibold leading-6 text-foreground"
						>
							Learn more <span aria-hidden="true">&rarr;</span>
						</a>
					</div>
				</div>
			</main>

			<footer className="border-t py-8">
				<div className="container mx-auto px-4 text-center text-sm text-muted-foreground">
					&copy; {new Date().getFullYear()} Duplex Stream. All rights reserved.
				</div>
			</footer>
		</div>
	)
}
