import { createFileRoute, redirect } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'
import { useEffect } from 'react'

const checkAuthAndGetToken = createServerFn({ method: 'GET' }).handler(
	async () => {
		const auth = await getAuth()

		if (auth.user && auth.accessToken) {
			return { authenticated: true, token: auth.accessToken }
		}

		// Not logged in - get sign in URL
		const signInUrl = await getSignInUrl({ state: 'desktop=true' })
		return { authenticated: false, signInUrl }
	},
)

export const Route = createFileRoute('/auth/desktop')({
	loader: async () => {
		return checkAuthAndGetToken()
	},
	component: DesktopAuthPage,
})

function DesktopAuthPage() {
	const data = Route.useLoaderData()

	useEffect(() => {
		if (data.authenticated && data.token) {
			// Client-side redirect to desktop app
			window.location.href = `duplex://auth/callback?token=${encodeURIComponent(data.token)}`
		} else if (data.signInUrl) {
			// Redirect to WorkOS
			window.location.href = data.signInUrl
		}
	}, [data])

	return (
		<div className="flex min-h-screen items-center justify-center">
			<p className="text-muted-foreground">Redirecting to desktop app...</p>
		</div>
	)
}
