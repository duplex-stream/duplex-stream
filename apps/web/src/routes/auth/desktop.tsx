import { createFileRoute, redirect } from '@tanstack/react-router'
import { getAuth, getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/desktop')({
	loader: async () => {
		// Check if user is already authenticated
		const auth = await getAuth()

		if (auth.user && auth.accessToken) {
			// Already logged in - redirect directly to desktop app with token
			const callbackUrl = `duplex://auth/callback?token=${encodeURIComponent(auth.accessToken)}`
			throw redirect({ href: callbackUrl })
		}

		// Not logged in - redirect to WorkOS with desktop state
		const signInUrl = await getSignInUrl({ state: 'desktop=true' })
		throw redirect({ href: signInUrl })
	},
	component: () => null,
})
