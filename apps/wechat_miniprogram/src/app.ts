import { PropsWithChildren } from 'react';
import { useLaunch } from '@tarojs/taro';

import { bootstrapWechatLogin } from './sdk';
import './app.scss';

export default function App({ children }: PropsWithChildren) {
  useLaunch(() => {
    bootstrapWechatLogin().catch(() => undefined);
  });

  return children;
}
