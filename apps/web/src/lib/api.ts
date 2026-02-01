interface Decision {
	id: string
	title: string
	summary: string
	context?: string
	status: 'active' | 'archived'
	source?: {
		type: 'claude' | 'chatgpt' | 'other'
		conversationId?: string
	}
	tags?: string[]
	createdAt: string
	updatedAt: string
}

interface PaginatedResponse<T> {
	data: T[]
	total: number
	page: number
	pageSize: number
}

export class ApiClient {
	private baseUrl: string
	private accessToken: string

	constructor(baseUrl: string, accessToken: string) {
		this.baseUrl = baseUrl
		this.accessToken = accessToken
	}

	private async fetch<T>(
		path: string,
		options: RequestInit = {},
	): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			...options,
			headers: {
				'Content-Type': 'application/json',
				Authorization: `Bearer ${this.accessToken}`,
				...options.headers,
			},
		})

		if (!response.ok) {
			throw new Error(`API error: ${response.status}`)
		}

		return response.json()
	}

	async getDecisions(params: {
		page?: number
		pageSize?: number
		search?: string
	}): Promise<PaginatedResponse<Decision>> {
		const searchParams = new URLSearchParams()
		if (params.page) searchParams.set('page', params.page.toString())
		if (params.pageSize)
			searchParams.set('pageSize', params.pageSize.toString())
		if (params.search) searchParams.set('search', params.search)

		const query = searchParams.toString()
		return this.fetch(`/decisions${query ? `?${query}` : ''}`)
	}

	async getDecision(id: string): Promise<Decision> {
		return this.fetch(`/decisions/${id}`)
	}

	async updateDecision(
		id: string,
		data: Partial<Decision>,
	): Promise<Decision> {
		return this.fetch(`/decisions/${id}`, {
			method: 'PATCH',
			body: JSON.stringify(data),
		})
	}

	async archiveDecision(id: string): Promise<void> {
		await this.fetch(`/decisions/${id}`, {
			method: 'PATCH',
			body: JSON.stringify({ status: 'archived' }),
		})
	}

	async createWorkspace(name: string): Promise<{ id: string; name: string }> {
		return this.fetch('/workspaces', {
			method: 'POST',
			body: JSON.stringify({ name }),
		})
	}
}

export function createApiClient(accessToken: string): ApiClient {
	const baseUrl =
		process.env.API_BASE_URL || 'https://api.duplex.stream'
	return new ApiClient(baseUrl, accessToken)
}
