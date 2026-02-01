import { createStart } from '@tanstack/react-start'
import { authkitMiddleware } from '@workos/authkit-tanstack-react-start'

export const startInstance = createStart(() => ({
	requestMiddleware: [
		authkitMiddleware({
			middlewareAuth: {
				enabled: true,
				unauthenticatedPaths: [
					'/',
					'/auth/login',
					'/auth/callback',
					'/auth/desktop',
					'/auth/logout',
					'/onboarding',
					'/onboarding/desktop',
				],
			},
		}),
	],
}))
