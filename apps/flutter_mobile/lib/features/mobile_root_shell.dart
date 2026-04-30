import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import 'ai_screen.dart';
import 'plan_screen.dart';
import 'reports_screen.dart';
import 'study_screen.dart';
import 'today_shell_screen.dart';
import 'wrong_words_screen.dart';

class MobileRootShell extends StatefulWidget {
  const MobileRootShell({super.key, required this.appState});

  final AppState appState;

  @override
  State<MobileRootShell> createState() => _MobileRootShellState();
}

class _MobileRootShellState extends State<MobileRootShell> {
  int _mainIndex = 0;
  int _todayReloadSeed = 0;
  int _wrongReloadSeed = 0;
  int _reportsReloadSeed = 0;
  int _aiReloadSeed = 0;
  bool _aiGenerateOnOpen = false;
  bool _aiShowPassageFirst = false;

  bool _showingStudy = false;
  String _studyMode = 'newWord';
  ResumeSessionHint? _studyResumeHint;

  int get _bodyIndex => _showingStudy ? 5 : _mainIndex;

  Future<void> _switchToMain(int index) async {
    if (!mounted) return;
    final wasShowingStudy = _showingStudy;
    setState(() {
      _showingStudy = false;
      if (wasShowingStudy) {
        _todayReloadSeed++;
        _wrongReloadSeed++;
        _reportsReloadSeed++;
      } else if (index == 0 && _mainIndex != 0) {
        _todayReloadSeed++;
      }
      if (index == 4) {
        _aiGenerateOnOpen = false;
        _aiShowPassageFirst = false;
      }
      _mainIndex = index;
    });
  }

  Future<void> _openStudy(String mode, ResumeSessionHint? hint) async {
    if (!mounted) return;
    setState(() {
      _studyMode = mode;
      _studyResumeHint = hint;
      _showingStudy = true;
    });
  }

  Future<void> _openAi({
    bool generateOnOpen = false,
    bool showPassageFirst = false,
  }) async {
    if (!mounted) return;
    setState(() {
      _showingStudy = false;
      _mainIndex = 4;
      _aiReloadSeed++;
      _aiGenerateOnOpen = generateOnOpen;
      _aiShowPassageFirst = showPassageFirst;
    });
  }

  void _handleStudyClosed() {
    if (!mounted) return;
    setState(() {
      _showingStudy = false;
      _mainIndex = 0;
      _todayReloadSeed++;
      _wrongReloadSeed++;
      _reportsReloadSeed++;
    });
  }

  @override
  Widget build(BuildContext context) {
    final pages = [
      TodayShellScreen(
        key: ValueKey('today-$_todayReloadSeed'),
        appState: widget.appState,
        onOpenPlan: () => _switchToMain(1),
        onOpenStudy: _openStudy,
        onOpenWrongWords: () => _switchToMain(2),
        onOpenReports: () => _switchToMain(3),
        onOpenAi: _openAi,
      ),
      PlanScreen(key: const ValueKey('plan'), sdk: widget.appState.sdk),
      WrongWordsScreen(
        key: ValueKey('wrong-$_wrongReloadSeed'),
        sdk: widget.appState.sdk,
        onStartStudy: (mode) => _openStudy(mode, null),
      ),
      ReportsScreen(
        key: ValueKey('reports-$_reportsReloadSeed'),
        sdk: widget.appState.sdk,
      ),
      AiScreen(
        key: ValueKey('ai-$_aiReloadSeed'),
        sdk: widget.appState.sdk,
        generateOnOpen: _aiGenerateOnOpen,
        showPassageFirst: _aiShowPassageFirst,
      ),
      StudyScreen(
        key: ValueKey(
          'study-$_studyMode-${_studyResumeHint?.current ?? 0}-${_studyResumeHint?.word ?? ''}',
        ),
        sdk: widget.appState.sdk,
        mode: _studyMode,
        resumeHint: _studyResumeHint,
        onCloseToToday: _handleStudyClosed,
        onOpenStudyMode: (mode) => _openStudy(mode, null),
      ),
    ];

    return Scaffold(
      body: IndexedStack(index: _bodyIndex, children: pages),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _mainIndex,
        onDestinationSelected: (index) {
          _switchToMain(index);
        },
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.today_outlined),
            selectedIcon: Icon(Icons.today),
            label: 'Today',
          ),
          NavigationDestination(
            icon: Icon(Icons.tune_outlined),
            selectedIcon: Icon(Icons.tune),
            label: 'Plan',
          ),
          NavigationDestination(
            icon: Icon(Icons.menu_book_outlined),
            selectedIcon: Icon(Icons.menu_book),
            label: 'Wrong',
          ),
          NavigationDestination(
            icon: Icon(Icons.query_stats_outlined),
            selectedIcon: Icon(Icons.query_stats),
            label: 'Reports',
          ),
          NavigationDestination(
            icon: Icon(Icons.auto_awesome_outlined),
            selectedIcon: Icon(Icons.auto_awesome),
            label: 'AI',
          ),
        ],
      ),
    );
  }
}
