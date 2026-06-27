export default {
  pages: [
    'pages/today/index',
    'pages/study/index',
  ],
  subpackages: [
    {
      root: 'subpkg',
      pages: [
        'wrong-words/index',
        'reports/index',
        'leaderboard/index',
        'plan/index',
        'onboarding/index',
        'profile/index',
        'settings/index',
        'croc-bti/index',
        'account/index',
      ],
    },
  ],
  window: {
    backgroundTextStyle: 'light',
    enablePullDownRefresh: true,
    navigationStyle: 'custom',
    navigationBarBackgroundColor: '#fbf7ff',
    navigationBarTitleText: '',
    navigationBarTextStyle: 'black',
    backgroundColor: '#fbf7ff',
  },
};
