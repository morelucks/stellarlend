import {
  TransactionBuilder,
  Contract,
  xdr,
  Address,
  nativeToScVal,
  Account,
  BASE_FEE,
} from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import axios from 'axios';
import { config } from '../config';
import logger from '../utils/logger';
import { InternalServerError } from '../utils/errors';
import { TransactionResponse, LendingOperation } from '../types';

const CONTRACT_METHODS: Record<LendingOperation, string> = {
  deposit: 'deposit_collateral',
  borrow: 'borrow_asset',
  repay: 'repay_debt',
  withdraw: 'withdraw_collateral',
};

// Timeout generous enough for client-side signing (5 minutes)
const TX_TIMEOUT_SECONDS = 300;

export class StellarService {
  private horizonUrl: string;
  private sorobanRpcUrl: string;
  private networkPassphrase: string;
  private contractId: string;
  private sorobanServer: SorobanServer;

  constructor() {
    this.horizonUrl = config.stellar.horizonUrl;
    this.sorobanRpcUrl = config.stellar.sorobanRpcUrl;
    this.networkPassphrase = config.stellar.networkPassphrase;
    this.contractId = config.stellar.contractId;
    this.sorobanServer = new SorobanServer(this.sorobanRpcUrl);
  }

  async getAccount(address: string): Promise<Account> {
    try {
      const response = await axios.get(`${this.horizonUrl}/accounts/${address}`);
      return new Account(response.data.id, response.data.sequence);
    } catch (error) {
      logger.error('Failed to fetch account:', error);
      throw new InternalServerError('Failed to fetch account information');
    }
  }

  async buildUnsignedTransaction(
    operation: LendingOperation,
    userAddress: string,
    assetAddress: string | undefined,
    amount: string
  ): Promise<string> {
    try {
      const account = await this.getAccount(userAddress);
      const contract = new Contract(this.contractId);

      const params = [
        new Address(userAddress).toScVal(),
        assetAddress ? new Address(assetAddress).toScVal() : xdr.ScVal.scvVoid(),
        nativeToScVal(BigInt(amount), { type: 'i128' }),
      ];

      const tx = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.networkPassphrase,
      })
        .addOperation(contract.call(CONTRACT_METHODS[operation], ...params))
        .setTimeout(TX_TIMEOUT_SECONDS)
        .build();

      const preparedTx = await this.sorobanServer.prepareTransaction(tx);
      return preparedTx.toXDR();
    } catch (error) {
      logger.error(`Failed to build unsigned ${operation} transaction:`, error);
      throw new InternalServerError(`Failed to build ${operation} transaction`);
    }
  }

  async submitTransaction(txXdr: string): Promise<TransactionResponse> {
    try {
      const response = await axios.post(`${this.horizonUrl}/transactions`, { tx: txXdr });
      return {
        success: true,
        transactionHash: response.data.hash,
        status: 'success',
        ledger: response.data.ledger,
      };
    } catch (error: any) {
      logger.error('Transaction submission failed:', error);
      return {
        success: false,
        status: 'failed',
        error: error.response?.data?.extras?.result_codes || error.message,
      };
    }
  }

  async monitorTransaction(txHash: string, timeoutMs = 30000): Promise<TransactionResponse> {
    const startTime = Date.now();
    const pollInterval = 1000;

    while (Date.now() - startTime < timeoutMs) {
      try {
        const response = await axios.get(`${this.horizonUrl}/transactions/${txHash}`);
        if (response.data.successful) {
          return {
            success: true,
            transactionHash: txHash,
            status: 'success',
            ledger: response.data.ledger,
          };
        }
        return {
          success: false,
          transactionHash: txHash,
          status: 'failed',
          error: 'Transaction failed',
        };
      } catch (error: any) {
        if (error.response?.status === 404) {
          await new Promise((resolve) => setTimeout(resolve, pollInterval));
          continue;
        }
        logger.error('Error monitoring transaction:', error);
        throw new InternalServerError('Failed to monitor transaction');
      }
    }

    return {
      success: false,
      transactionHash: txHash,
      status: 'pending',
      message: 'Transaction monitoring timeout',
    };
  }

  async healthCheck(): Promise<{ horizon: boolean; sorobanRpc: boolean }> {
    const results = { horizon: false, sorobanRpc: false };

    try {
      await axios.get(`${this.horizonUrl}/`);
      results.horizon = true;
    } catch (error) {
      logger.error('Horizon health check failed:', error);
    }

    try {
      await this.sorobanServer.getHealth();
      results.sorobanRpc = true;
    } catch (error) {
      logger.error('Soroban RPC health check failed:', error);
    }

    return results;
  }
}
