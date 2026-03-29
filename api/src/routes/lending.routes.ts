import { Router } from 'express';
import * as lendingController from '../controllers/lending.controller';
import { prepareValidation, submitValidation } from '../middleware/validation';

const router = Router();

// v2: client-side signing flow
router.get('/prepare/:operation', prepareValidation, lendingController.prepare);
router.post('/submit', submitValidation, lendingController.submit);

export default router;
