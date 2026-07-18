import 'package:flutter/material.dart';

enum LearningContentMode { wordStudy, examPractice }

class LearningContentModeSelector extends StatelessWidget {
  const LearningContentModeSelector({
    super.key,
    required this.value,
    required this.onChanged,
  });

  final LearningContentMode value;
  final ValueChanged<LearningContentMode> onChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<LearningContentMode>(
      segments: const [
        ButtonSegment(
          value: LearningContentMode.wordStudy,
          icon: Icon(Icons.spellcheck_rounded),
          label: Text('单词学习'),
        ),
        ButtonSegment(
          value: LearningContentMode.examPractice,
          icon: Icon(Icons.assignment_outlined),
          label: Text('模拟练习'),
        ),
      ],
      selected: {value},
      onSelectionChanged: (selected) => onChanged(selected.first),
      showSelectedIcon: false,
    );
  }
}
