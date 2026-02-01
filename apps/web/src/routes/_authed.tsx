import { createFileRoute, Outlet, redirect, Link } from '@tanstack/react-router'
import { getAuth } from '@workos/authkit-tanstack-react-start'

export interface AuthContext {
	user: {
		id: string
		email: string
		firstName?: string
		lastName?: string
		profilePictureUrl?: string
	}
}

export const Route = createFileRoute('/_authed')({
	beforeLoad: async () => {
		const auth = await getAuth()

		if (!auth.user) {
			throw redirect({ to: '/auth/login' })
		}

		return {
			user: {
				id: auth.user.id,
				email: auth.user.email,
				firstName: auth.user.firstName ?? undefined,
				lastName: auth.user.lastName ?? undefined,
				profilePictureUrl: auth.user.profilePictureUrl ?? undefined,
			},
		} as AuthContext
	},
	component: AuthedLayout,
})

function AuthedLayout() {
	const { user } = Route.useRouteContext()

	return (
		<div className="flex min-h-screen flex-col">
			<header className="border-b">
				<div className="container mx-auto flex h-16 items-center justify-between px-4">
					<Link to="/" className="text-xl font-bold">
						Duplex Stream
					</Link>
					<nav className="flex items-center gap-4">
						<Link
							to="/dashboard"
							className="text-sm font-medium text-muted-foreground hover:text-foreground"
						>
							Dashboard
						</Link>
						<Link
							to="/settings"
							className="text-sm font-medium text-muted-foreground hover:text-foreground"
						>
							Settings
						</Link>
						<div className="flex items-center gap-2">
							{user.profilePictureUrl ? (
								<img
									src={user.profilePictureUrl}
									alt={user.firstName || user.email}
									className="h-8 w-8 rounded-full"
								/>
							) : (
								<div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground">
									{(user.firstName?.[0] || user.email[0]).toUpperCase()}
								</div>
							)}
							<span className="text-sm font-medium">
								{user.firstName || user.email}
							</span>
						</div>
						<Link
							to="/auth/logout"
							className="text-sm font-medium text-muted-foreground hover:text-foreground"
						>
							Sign Out
						</Link>
					</nav>
				</div>
			</header>
			<main className="flex-1">
				<Outlet />
			</main>
		</div>
	)
}
