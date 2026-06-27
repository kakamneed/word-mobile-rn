import { ReactNode } from 'react';
import { Button, Text } from '@tarojs/components';

import './PrimaryButton.scss';

interface PrimaryButtonProps {
  children: ReactNode;
  disabled?: boolean;
  variant?: 'primary' | 'secondary';
  onClick?: () => void;
}

export function PrimaryButton({
  children,
  disabled = false,
  variant = 'primary',
  onClick,
}: PrimaryButtonProps) {
  return (
    <Button
      className={`primary-button primary-button--${variant}`}
      disabled={disabled}
      onClick={onClick}
    >
      {typeof children === 'string' || typeof children === 'number' ? (
        <Text>{children}</Text>
      ) : (
        children
      )}
    </Button>
  );
}
