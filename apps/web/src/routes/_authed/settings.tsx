import { createFileRoute } from '@tanstack/react-router'
import { Button } from '~/components/ui/button'

export const Route = createFileRoute('/_authed/settings')({
	component: SettingsPage,
})

function SettingsPage() {
	const { user } = Route.useRouteContext()

	return (
		<div className="container mx-auto px-4 py-8">
			<h1 className="mb-8 text-3xl font-bold">Settings</h1>

			<div className="max-w-2xl space-y-8">
				<section className="rounded-lg border p-6">
					<h2 className="text-xl font-semibold">Profile</h2>
					<div className="mt-4 space-y-4">
						<div className="flex items-center gap-4">
							{user.profilePictureUrl ? (
								<img
									src={user.profilePictureUrl}
									alt={user.firstName || user.email}
									className="h-16 w-16 rounded-full"
								/>
							) : (
								<div className="flex h-16 w-16 items-center justify-center rounded-full bg-primary text-2xl text-primary-foreground">
									{(user.firstName?.[0] || user.email[0]).toUpperCase()}
								</div>
							)}
							<div>
								<p className="font-medium">
									{user.firstName} {user.lastName}
								</p>
								<p className="text-sm text-muted-foreground">{user.email}</p>
							</div>
						</div>
					</div>
				</section>

				<section className="rounded-lg border p-6">
					<h2 className="text-xl font-semibold">Desktop App</h2>
					<p className="mt-2 text-sm text-muted-foreground">
						Install the Duplex Stream desktop app to automatically capture
						decisions from your AI conversations.
					</p>
					<div className="mt-4">
						<Button variant="outline">Download for macOS</Button>
					</div>
				</section>

				<section className="rounded-lg border p-6">
					<h2 className="text-xl font-semibold">Danger Zone</h2>
					<p className="mt-2 text-sm text-muted-foreground">
						Permanently delete your account and all associated data.
					</p>
					<div className="mt-4">
						<Button variant="destructive">Delete Account</Button>
					</div>
				</section>
			</div>
		</div>
	)
}
