import { cloudflare } from '@cloudflare/vite-plugin'
import tailwindcss from '@tailwindcss/vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import tsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
	server: {
		port: 5173,
		watch: {
			// Ignore generated route tree to prevent HMR loop
			ignored: ['**/routeTree.gen.ts'],
		},
	},
	plugins: [
		tailwindcss(),
		tsconfigPaths({
			projects: ['./tsconfig.json'],
		}),
		cloudflare({ viteEnvironment: { name: 'ssr' } }),
		tanstackStart(),
		viteReact(),
	],
})
