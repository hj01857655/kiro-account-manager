// Unified token management structure
export interface Token {
  // Core token info
  accessToken?: string;
  refreshToken: string;
  expiresAt?: Date;
  
  // API response fields
  expiresIn?: number;
  profileArn?: string;
}

// Token refresh response structure supporting both Social and IdC auth
export interface RefreshResponse {
  accessToken: string;
  expiresIn: number;
  refreshToken?: string;
  
  // Social auth specific fields
  profileArn?: string;
  
  // IdC auth specific fields
  tokenType?: string;
  
  // Optional response fields
  originSessionId?: string;
  issuedTokenType?: string;
  aws_sso_app_session_id?: string;
  idToken?: string;
}

// Social auth refresh request structure
export interface RefreshRequest {
  refreshToken: string;
}

// IdC auth refresh request structure
export interface IdcRefreshRequest {
  clientId: string;
  clientSecret: string;
  grantType: string;
  refreshToken: string;
}

// Token utility functions
export function isTokenExpired(token: Token): boolean {
  if (!token.expiresAt) return false;
  return new Date() > token.expiresAt;
}

export function createTokenFromRefreshResponse(
  resp: RefreshResponse,
  originalRefreshToken: string
): Token {
  return {
    accessToken: resp.accessToken,
    refreshToken: originalRefreshToken,
    expiresIn: resp.expiresIn,
    profileArn: resp.profileArn,
    expiresAt: new Date(Date.now() + resp.expiresIn * 1000),
  };
}
