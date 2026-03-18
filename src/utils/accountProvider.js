export function isGitHubProvider(provider) {
  return provider === 'Github' || provider === 'GitHub'
}

export function normalizeProviderId(provider) {
  return isGitHubProvider(provider) ? 'Github' : provider
}

export function getProviderDisplayName(provider) {
  return isGitHubProvider(provider) ? 'Github' : provider
}
