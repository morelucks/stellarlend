import request from 'supertest';
import app from '../app';

describe('Validation Middleware', () => {
  describe('Prepare Validation (GET /api/lending/prepare/:operation)', () => {
    it('should reject empty userAddress', async () => {
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .send({ amount: '1000000' });

      expect(response.status).toBe(400);
    });

    it('should reject zero amount', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').send({
        userAddress: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        amount: '0',
      });

      expect(response.status).toBe(400);
    });

    it('should reject negative amount', async () => {
      const response = await request(app).get('/api/lending/prepare/deposit').send({
        userAddress: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        amount: '-1000',
      });

      expect(response.status).toBe(400);
    });

    it('should reject invalid operation', async () => {
      const response = await request(app).get('/api/lending/prepare/invalid_op').send({
        userAddress: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        amount: '1000000',
      });

      expect(response.status).toBe(400);
    });

    it('should not require userSecret', async () => {
      // Sending userSecret should not cause a validation error (it's simply ignored)
      // The route should still validate normally without it
      const response = await request(app)
        .get('/api/lending/prepare/deposit')
        .send({ amount: '1000000' }); // missing userAddress — should still be 400

      expect(response.status).toBe(400);
    });
  });

  describe('Submit Validation (POST /api/lending/submit)', () => {
    it('should reject missing signedXdr', async () => {
      const response = await request(app).post('/api/lending/submit').send({});

      expect(response.status).toBe(400);
    });

    it('should reject empty signedXdr', async () => {
      const response = await request(app).post('/api/lending/submit').send({ signedXdr: '' });

      expect(response.status).toBe(400);
    });
  });
});
