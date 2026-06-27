declare const require: (id: string) => any;
declare const __dirname: string;

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');

function read(relativePath: string) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function assert(condition: unknown, message: string) {
  if (!condition) throw new Error(message);
}

const appConfig = read('src/app.config.ts');
const screenSource = read('src/components/Screen.tsx');
const primaryButtonScss = read('src/components/PrimaryButton.scss');
const today = read('src/pages/today/index.tsx');
const todayConfig = read('src/pages/today/index.config.ts');
const todayScss = read('src/pages/today/index.scss');
const plan = read('src/subpkg/plan/index.tsx');
const planScss = read('src/subpkg/plan/index.scss');
const wrong = read('src/subpkg/wrong-words/index.tsx');
const reports = read('src/subpkg/reports/index.tsx');
const reportsScss = read('src/subpkg/reports/index.scss');
const study = read('src/pages/study/index.tsx');
const account = read('src/subpkg/account/index.tsx');
const leaderboard = read('src/subpkg/leaderboard/index.tsx');
const onboarding = read('src/subpkg/onboarding/index.tsx');
const profile = read('src/subpkg/profile/index.tsx');
const settings = read('src/subpkg/settings/index.tsx');
const themeStore = read('src/sdk/themeStore.ts');
const crocAnimation = read('src/components/CrocodileFrameAnimation.tsx');
const sourceFiles = [screenSource, today, plan, wrong, reports, study].join('\n');

assert(!appConfig.includes('tabBar'), 'Native tabBar should not be configured.');
assert(appConfig.includes("navigationStyle: 'custom'"), 'App must use custom navigation style.');
assert(sourceFiles.includes('activeTab="today"'), 'Today must use custom bottom navigation.');
assert(sourceFiles.includes('activeTab="plan"'), 'Plan must use custom bottom navigation.');
assert(sourceFiles.includes('activeTab="wrong"'), 'Wrong Words must use custom bottom navigation.');
assert(sourceFiles.includes('activeTab="reports"'), 'Reports must use custom bottom navigation.');

const tabLabels = [...screenSource.matchAll(/label: '([^']+)'/g)].map((match) => match[1]);
assert(tabLabels.length >= 4, 'Bottom nav labels must be declared.');
assert(new Set(tabLabels.slice(0, 4)).size === 4, 'Bottom nav labels must be unique.');
assert(screenSource.includes("label: '今日'") && screenSource.includes("label: '计划'"), 'Bottom nav labels must be readable Chinese.');
assert(!screenSource.includes('浠婃棩') && !screenSource.includes('鈥'), 'Screen source must not contain mojibake text.');
assert(screenSource.includes('navigateMain') && screenSource.includes('Taro.redirectTo'), 'Bottom nav must replace the current page instead of stacking navigateTo pages.');

assert(screenSource.includes('/subpkg/croc-bti/index'), 'Croc BTI must be reachable from drawer.');
assert(screenSource.includes('/subpkg/leaderboard/index'), 'Leaderboard must be reachable from drawer.');
assert(screenSource.includes('/subpkg/onboarding/index'), 'Beginner guide must open onboarding, not Plan.');
assert(screenSource.includes('/subpkg/profile/index'), 'Profile drawer item must open profile page.');
assert(screenSource.includes('/subpkg/settings/index'), 'Settings drawer item must open settings page.');
assert(screenSource.includes('screen__back') && screenSource.includes('navigateBack'), 'Subpages without tab bar must render a back button.');
assert(screenSource.includes('getStoredTheme'), 'Screen must apply persisted theme variables.');
assert(screenSource.includes('AppIcon'), 'Screen navigation must use AppIcon.');
assert(!screenSource.includes("label: 'AI'"), 'AI tab must not exist.');
assert(!screenSource.includes("label: 'Account'"), 'Account must not be a bottom tab.');

assert(appConfig.includes('onboarding/index'), 'Onboarding page must be registered in subpackage config.');
assert(appConfig.includes('profile/index'), 'Profile page must be registered in subpackage config.');
assert(appConfig.includes('settings/index'), 'Settings page must be registered in subpackage config.');
assert(!appConfig.includes('/pages/ai/'), 'AI page must not be linked from app config.');

assert(primaryButtonScss.includes('var(--flutter-primary') && primaryButtonScss.includes('var(--flutter-soft'), 'PrimaryButton must follow persisted theme variables.');
assert(todayScss.includes('var(--flutter-hero-purple') && todayScss.includes('var(--flutter-primary'), 'Today page must follow persisted theme variables.');
assert(themeStore.includes('Taro.setStorageSync') && themeStore.includes('app_color_style'), 'Theme store must persist color style.');
assert(themeStore.includes('鳄甲紫') && !themeStore.includes('槌勭敳'), 'Theme store labels must stay readable Chinese.');
assert(settings.includes('设置') && settings.includes('主题和偏好'), 'Settings page must own theme/preferences UI.');
assert(settings.includes('saveStoredTheme') && settings.includes('主题已保存'), 'Settings page must persist selected theme.');

assert(!today.includes('AI'), 'Today must not contain AI summary/generation UI.');
assert(today.includes('useDidShow'), 'Today must refresh when returning from Study.');
assert(today.includes('refreshing'), 'Today must expose a visible refresh state.');
assert(today.includes('today-refresh-overlay'), 'Today must render the refresh overlay indicator.');
assert(today.includes('CrocodileRefreshPopup'), 'Today refresh must use the crocodile frame refresh popup.');
assert(todayConfig.includes('enablePullDownRefresh: true'), 'Today page config must enable pull-down refresh.');
assert(crocAnimation.includes('crocodile_refresh/crocodile_crawl_frame_01.png'), 'Refresh popup must use compressed Flutter crawl frame assets.');
assert(crocAnimation.includes('crocodile_crawl_frame_06.png'), 'Refresh popup must include the full six-frame crawl animation.');
assert(crocAnimation.includes("label = '刷新中'"), 'Refresh popup label must be readable Chinese.');
assert(!todayScss.includes('today-refresh-slide'), 'Today refresh must not use the old bar placeholder animation.');

assert(reports.includes('modeSeries'), 'Reports page must use modeSeries.');
assert(reports.includes('selectedMode'), 'Reports page must support selectedMode.');
assert(reports.includes('chart.segments.map'), 'Reports chart must render native mini-program line segments.');
assert(reports.includes('line-chart__segment'), 'Reports chart must render visible line segments.');
assert(!reports.includes('dangerouslySetInnerHTML'), 'Reports chart must not rely on innerHTML SVG in WeChat.');
assert(reportsScss.includes('transform-origin: 0 50%'), 'Reports chart line segments must rotate around the start point.');

assert(study.includes('Swiper'), 'Study page must render vertical feed with Swiper.');
assert(study.includes('vertical'), 'Study Swiper must be vertical.');
assert(study.includes('feedItems'), 'Study page must build feedItems.');
assert(study.includes('StudyIcon'), 'Study side actions must be icon-based.');

assert(plan.includes('savePlan'), 'Plan page must call savePlan.');
assert(plan.includes('applySavedPlanToToday'), 'Plan page must call applySavedPlanToToday.');
assert(plan.includes('growthRulesByMode'), 'Plan page must expose per-mode growth rules.');
assert(plan.includes('RuleEditor'), 'Plan page must render editable growth rule controls.');
assert(planScss.includes('.stepper__button'), 'Plan stepper CSS must match current stepper markup.');
assert(planScss.includes('grid-template-columns: 42px 72px 42px'), 'Plan steppers must keep Flutter-like horizontal layout.');
assert(plan.includes('radio-dot') && planScss.includes('.radio-dot--active'), 'Wordbook management must render selectable radio rows.');
assert(plan.includes('词书管理') && plan.includes('保存计划'), 'Plan page source text must stay readable Chinese.');
assert(!planScss.includes('position: fixed'), 'Plan action buttons must stay in scroll flow, not fixed above nav.');

assert(wrong.includes('WrongDetailCard'), 'Wrong Words must render detail in a separate structured card.');
assert(wrong.includes('wrong-detail-section'), 'Wrong Words detail must have separated sections.');
assert(onboarding.includes('saveAndApply'), 'Onboarding must save the plan and sync it to Today.');
assert(account.includes('绑定状态') && account.includes('侧边栏功能'), 'Account page visible language must be Chinese.');
assert(!account.includes('Bindings') && !account.includes('Drawer actions'), 'Account page must not show English placeholder headings.');
assert(profile.includes('个人信息') && profile.includes('昵称') && profile.includes('头像颜色'), 'Profile page must own nickname/avatar UI.');
assert(leaderboard.includes('题数榜') && leaderboard.includes('周榜'), 'Leaderboard filters must be Chinese.');
assert(leaderboard.includes('leaderboard-avatar__rank'), 'Leaderboard rows must follow Flutter avatar/rank badge structure.');

console.log('UI contract checks passed.');
