import { StellarService } from '../services/stellar.service';
import axios from 'axios';

jest.mock('axios');

const mockPreparedTx = {
  sign: jest.fn(),
  toXDR: jest.fn().mockReturnValue('unsigned_tx_xdr'),
};

const mockSorobanServer = {
  prepareTransaction: jest.fn().mockResolvedValue(mockPreparedTx),
  getHealth: jest.fn().mockResolvedValue({}),
};

const mockTxBuilder = {
  addOperation: jest.fn().mockReturnThis(),
  setTimeout: jest.fn().mockReturnThis(),
  build: jest.fn().mockReturnValue({}),
};

jest.mock('@stellar/stellar-sdk', () => ({
  Account: jest.fn().mockImplementation(() => ({
    accountId: jest.fn().mockReturnValue('GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'),
    sequenceNumber: jest.fn().mockReturnValue('123456789'),
    incrementSequenceNumber: jest.fn(),
  })),
  TransactionBuilder: jest.fn().mockImplementation(() => mockTxBuilder),
  Contract: jest.fn().mockImplementation(() => ({
    call: jest.fn().mockReturnValue({}),
  })),
  Address: jest.fn().mockImplementation(() => ({ toScVal: jest.fn().mockReturnValue({}) })),
  nativeToScVal: jest.fn().mockReturnValue({}),
  BASE_FEE: '100',
  Networks: { TESTNET: 'Test SDF Network ; September 2015' },
  xdr: { ScVal: { scvVoid: jest.fn().mockReturnValue({}) } },
}));

jest.mock('@stellar/stellar-sdk/rpc', () => ({
  Server: jest.fn().mockImplementation(() => mockSorobanServer),
}));

const mockedAxios = axios as jest.Mocked<typeof axios>;

describe('StellarService', () => {
  let service: StellarService;

  beforeEach(() => {
    service = new StellarService();
    jest.clearAllMocks();
    mockSorobanServer.prepareTransaction.mockResolvedValue(mockPreparedTx);
    mockSorobanServer.getHealth.mockResolvedValue({});
  });

  describe('getAccount', () => {
    it('should fetch account information', async () => {
      mockedAxios.get.mockResolvedValue({
        data: {
          id: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
          sequence: '123456789',
        },
      });

      const account = await service.getAccount(
        'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'
      );

      expect(account).toBeDefined();
      expect(mockedAxios.get).toHaveBeenCalledWith(
        expect.stringContaining('/accounts/GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX')
      );
    });

    it('should throw error when account fetch fails', async () => {
      mockedAxios.get.mockRejectedValue(new Error('Network error'));
      await expect(service.getAccount('invalid_address')).rejects.toThrow();
    });
  });

  describe('submitTransaction', () => {
    it('should submit transaction successfully', async () => {
      mockedAxios.post.mockResolvedValue({
        data: { hash: 'tx_hash_123', ledger: 12345, successful: true },
      });

      const result = await service.submitTransaction('mock_tx_xdr');

      expect(result.success).toBe(true);
      expect(result.transactionHash).toBe('tx_hash_123');
      expect(result.ledger).toBe(12345);
    });

    it('should handle transaction submission failure', async () => {
      mockedAxios.post.mockRejectedValue({
        response: { data: { extras: { result_codes: { transaction: 'tx_failed' } } } },
      });

      const result = await service.submitTransaction('mock_tx_xdr');

      expect(result.success).toBe(false);
      expect(result.status).toBe('failed');
    });
  });

  describe('monitorTransaction', () => {
    it('should monitor transaction until success', async () => {
      mockedAxios.get.mockResolvedValue({ data: { successful: true, ledger: 12345 } });

      const result = await service.monitorTransaction('tx_hash_123');

      expect(result.success).toBe(true);
      expect(result.transactionHash).toBe('tx_hash_123');
      expect(result.status).toBe('success');
    });

    it('should timeout if transaction takes too long', async () => {
      mockedAxios.get.mockRejectedValue({ response: { status: 404 } });

      const result = await service.monitorTransaction('tx_hash_123', 2000);

      expect(result.success).toBe(false);
      expect(result.status).toBe('pending');
    });

    it('should handle failed transaction', async () => {
      mockedAxios.get.mockResolvedValue({ data: { successful: false } });

      const result = await service.monitorTransaction('tx_hash_123');

      expect(result.success).toBe(false);
      expect(result.status).toBe('failed');
    });
  });

  describe('healthCheck', () => {
    it('should return healthy status for all services', async () => {
      mockedAxios.get.mockResolvedValue({ data: {} });
      mockSorobanServer.getHealth.mockResolvedValue({});

      const result = await service.healthCheck();

      expect(result.horizon).toBe(true);
    });

    it('should return unhealthy status when services fail', async () => {
      mockedAxios.get.mockRejectedValue(new Error('Connection failed'));
      mockSorobanServer.getHealth.mockRejectedValue(new Error('Connection failed'));

      const result = await service.healthCheck();

      expect(result.horizon).toBe(false);
      expect(result.sorobanRpc).toBe(false);
    });
  });

  describe('buildUnsignedTransaction', () => {
    it.each(['deposit', 'borrow', 'repay', 'withdraw'] as const)(
      'should build unsigned %s transaction without requiring a secret key',
      async (operation) => {
        mockedAxios.get.mockResolvedValue({
          data: {
            id: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
            sequence: '123456789',
          },
        });

        const result = await service.buildUnsignedTransaction(
          operation,
          'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
          undefined,
          '1000000'
        );

        expect(result).toBe('unsigned_tx_xdr');
        expect(mockSorobanServer.prepareTransaction).toHaveBeenCalled();
      }
    );
  });
});
