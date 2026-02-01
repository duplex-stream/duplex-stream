import { createFileRoute, redirect } from '@tanstack/react-router'
import { getSignInUrl } from '@workos/authkit-tanstack-react-start'

export const Route = createFileRoute('/auth/desktop')({
	loader: async () => {
		// Use state parameter to indicate desktop flow
		const signInUrl = await getSignInUrl({ state: 'desktop=true' })
		throw redirect({ href: signInUrl })
	},
	component: () => null,
})
