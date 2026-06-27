import { Image, Text, View } from '@tarojs/components';

import './CrocodileFrameAnimation.scss';

const crawlFrames = [
  '/assets/crocodile_refresh/crocodile_crawl_frame_01.png',
  '/assets/crocodile_refresh/crocodile_crawl_frame_02.png',
  '/assets/crocodile_refresh/crocodile_crawl_frame_03.png',
  '/assets/crocodile_refresh/crocodile_crawl_frame_04.png',
  '/assets/crocodile_refresh/crocodile_crawl_frame_05.png',
  '/assets/crocodile_refresh/crocodile_crawl_frame_06.png',
];

export function CrocodileFrameAnimation({ className = '' }: { className?: string }) {
  return (
    <View className={`croc-frame croc-frame--crawl ${className}`}>
      {crawlFrames.map((src, index) => (
        <Image
          key={src}
          className={`croc-frame__image croc-frame__image--${index + 1}`}
          src={src}
          mode="aspectFit"
        />
      ))}
    </View>
  );
}

export function CrocodileRefreshPopup({
  active,
  label = '刷新中',
}: {
  active: boolean;
  label?: string;
}) {
  if (!active) return null;
  return (
    <View className="croc-refresh-popup">
      <CrocodileFrameAnimation />
      <Text className="croc-refresh-popup__label">{label}</Text>
    </View>
  );
}
