import { body, param, validationResult } from 'express-validator';
import { Request, Response, NextFunction } from 'express';
import { ValidationError } from '../utils/errors';

const VALID_OPERATIONS = ['deposit', 'borrow', 'repay', 'withdraw'];

export const validateRequest = (req: Request, res: Response, next: NextFunction) => {
  const errors = validationResult(req);
  if (!errors.isEmpty()) {
    const errorMessages = errors
      .array()
      .map((err) => err.msg)
      .join(', ');
    throw new ValidationError(errorMessages);
  }
  next();
};

const amountValidation = body('amount')
  .isString()
  .notEmpty()
  .withMessage('Amount is required')
  .custom((value) => {
    const num = BigInt(value);
    return num > 0n;
  })
  .withMessage('Amount must be greater than zero');

export const prepareValidation = [
  param('operation')
    .isIn(VALID_OPERATIONS)
    .withMessage(`Operation must be one of: ${VALID_OPERATIONS.join(', ')}`),
  body('userAddress').isString().notEmpty().withMessage('User address is required'),
  amountValidation,
  body('assetAddress').optional().isString(),
  validateRequest,
];

export const submitValidation = [
  body('signedXdr').isString().notEmpty().withMessage('signedXdr is required'),
  validateRequest,
];

// Kept for backward compatibility — deprecated, will be removed in v2
export const depositValidation = prepareValidation;
export const borrowValidation = prepareValidation;
export const repayValidation = prepareValidation;
export const withdrawValidation = prepareValidation;
