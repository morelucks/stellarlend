export interface DepositRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface BorrowRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface RepayRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface WithdrawRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export type LendingOperation = 'deposit' | 'borrow' | 'repay' | 'withdraw';

export interface PrepareRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface PrepareResponse {
  unsignedXdr: string;
  operation: LendingOperation;
  expiresAt: string;
}

export interface SubmitRequest {
  signedXdr: string;
}

export interface TransactionResponse {
  success: boolean;
  transactionHash?: string;
  status: 'pending' | 'success' | 'failed';
  message?: string;
  error?: string;
  ledger?: number;
}

export interface PositionResponse {
  userAddress: string;
  collateral: string;
  debt: string;
  borrowInterest: string;
  lastAccrualTime: number;
  collateralRatio?: string;
}

export interface HealthCheckResponse {
  status: 'healthy' | 'unhealthy';
  timestamp: string;
  services: {
    horizon: boolean;
    sorobanRpc: boolean;
  };
}

export enum TransactionStatus {
  PENDING = 'pending',
  SUCCESS = 'success',
  FAILED = 'failed',
  NOT_FOUND = 'not_found',
}
