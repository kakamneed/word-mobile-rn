import 'package:flutter/material.dart';

import '../sdk/sdk.dart';

Future<bool> showExamPaperImportReviewDialog(
  BuildContext context,
  ExamPaperImportDraft draft,
) async {
  return await showDialog<bool>(
        context: context,
        barrierDismissible: false,
        builder: (context) => _ExamPaperImportReviewDialog(draft: draft),
      ) ??
      false;
}

class _ExamPaperImportReviewDialog extends StatefulWidget {
  const _ExamPaperImportReviewDialog({required this.draft});

  final ExamPaperImportDraft draft;

  @override
  State<_ExamPaperImportReviewDialog> createState() =>
      _ExamPaperImportReviewDialogState();
}

class _ExamPaperImportReviewDialogState
    extends State<_ExamPaperImportReviewDialog> {
  late final TextEditingController _id;
  late final TextEditingController _exam;
  late final TextEditingController _title;
  late final TextEditingController _year;
  late final TextEditingController _passage;
  late final TextEditingController _stem;
  late final TextEditingController _choices;
  late final TextEditingController _answer;
  int _sectionIndex = 0;
  int _questionIndex = 0;

  Map<String, dynamic> get _paper => widget.draft.paper;
  List<dynamic> get _sections =>
      _paper['sections'] as List<dynamic>? ?? const [];
  Map<String, dynamic>? get _section => _sections.isEmpty
      ? null
      : _sections[_sectionIndex.clamp(0, _sections.length - 1)]
            as Map<String, dynamic>;
  List<dynamic> get _questions =>
      _section?['questions'] as List<dynamic>? ?? const [];
  Map<String, dynamic>? get _question => _questions.isEmpty
      ? null
      : _questions[_questionIndex.clamp(0, _questions.length - 1)]
            as Map<String, dynamic>;

  @override
  void initState() {
    super.initState();
    _id = TextEditingController(text: '${_paper['id'] ?? ''}');
    _exam = TextEditingController(text: '${_paper['exam'] ?? ''}');
    _title = TextEditingController(text: '${_paper['title'] ?? ''}');
    _year = TextEditingController(text: '${_paper['year'] ?? ''}');
    _passage = TextEditingController();
    _stem = TextEditingController();
    _choices = TextEditingController();
    _answer = TextEditingController();
    _loadSelection();
  }

  void _loadSelection() {
    _passage.text = '${_section?['passage'] ?? ''}';
    _stem.text = '${_question?['stem'] ?? ''}';
    final choices = _question?['choices'] as List<dynamic>? ?? const [];
    _choices.text = choices
        .map((item) {
          final choice = item as Map<String, dynamic>;
          return '${choice['label'] ?? ''}: ${choice['text'] ?? ''}';
        })
        .join('\n');
    _answer.text = '${_question?['answer'] ?? ''}';
  }

  void _saveSelection() {
    _paper['id'] = _id.text.trim();
    _paper['exam'] = _exam.text.trim();
    _paper['title'] = _title.text.trim();
    _paper['year'] = int.tryParse(_year.text.trim()) ?? 0;
    final section = _section;
    if (section != null) section['passage'] = _passage.text;
    final question = _question;
    if (question == null) return;
    question['stem'] = _stem.text;
    question['answer'] = _answer.text.trim().isEmpty
        ? null
        : _answer.text.trim().toUpperCase();
    question['choices'] = _choices.text
        .split('\n')
        .map((line) => line.trim())
        .where((line) => line.isNotEmpty && line.contains(':'))
        .map((line) {
          final separator = line.indexOf(':');
          return {
            'label': line.substring(0, separator).trim().toUpperCase(),
            'text': line.substring(separator + 1).trim(),
          };
        })
        .toList(growable: false);
  }

  void _selectSection(int index) {
    _saveSelection();
    setState(() {
      _sectionIndex = index;
      _questionIndex = 0;
      _loadSelection();
    });
  }

  void _selectQuestion(int index) {
    _saveSelection();
    setState(() {
      _questionIndex = index;
      _loadSelection();
    });
  }

  @override
  void dispose() {
    for (final controller in [
      _id,
      _exam,
      _title,
      _year,
      _passage,
      _stem,
      _choices,
      _answer,
    ]) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('审阅试卷草稿'),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '来源：${widget.draft.provenance['sourceName'] ?? '未知'} · 原图未保留',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              for (final warning in widget.draft.warnings)
                Text('校验提示：$warning'),
              const SizedBox(height: 12),
              TextField(
                key: const ValueKey('exam-import-id'),
                controller: _id,
                decoration: const InputDecoration(labelText: '试卷 ID'),
              ),
              TextField(
                controller: _exam,
                decoration: const InputDecoration(labelText: '考试类型'),
              ),
              TextField(
                controller: _title,
                decoration: const InputDecoration(labelText: '试卷标题'),
              ),
              TextField(
                controller: _year,
                keyboardType: TextInputType.number,
                decoration: const InputDecoration(labelText: '年份'),
              ),
              if (_sections.isNotEmpty) ...[
                DropdownButtonFormField<int>(
                  initialValue: _sectionIndex,
                  decoration: const InputDecoration(labelText: '部分'),
                  items: [
                    for (var index = 0; index < _sections.length; index++)
                      DropdownMenuItem(
                        value: index,
                        child: Text(
                          '${(_sections[index] as Map)['title'] ?? index + 1}',
                        ),
                      ),
                  ],
                  onChanged: (value) {
                    if (value != null) _selectSection(value);
                  },
                ),
                TextField(
                  key: const ValueKey('exam-import-passage'),
                  controller: _passage,
                  minLines: 3,
                  maxLines: 8,
                  decoration: const InputDecoration(labelText: '文章'),
                ),
              ],
              if (_questions.isNotEmpty) ...[
                DropdownButtonFormField<int>(
                  initialValue: _questionIndex,
                  decoration: const InputDecoration(labelText: '题目'),
                  items: [
                    for (var index = 0; index < _questions.length; index++)
                      DropdownMenuItem(
                        value: index,
                        child: Text('第 ${index + 1} 题'),
                      ),
                  ],
                  onChanged: (value) {
                    if (value != null) _selectQuestion(value);
                  },
                ),
                TextField(
                  key: const ValueKey('exam-import-stem'),
                  controller: _stem,
                  minLines: 2,
                  maxLines: 5,
                  decoration: const InputDecoration(labelText: '题干'),
                ),
                TextField(
                  key: const ValueKey('exam-import-choices'),
                  controller: _choices,
                  minLines: 3,
                  maxLines: 6,
                  decoration: const InputDecoration(
                    labelText: '选项',
                    helperText: '每行使用 A: 选项内容',
                  ),
                ),
                TextField(
                  key: const ValueKey('exam-import-answer'),
                  controller: _answer,
                  decoration: const InputDecoration(labelText: '答案（可留空）'),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('取消'),
        ),
        FilledButton.icon(
          key: const ValueKey('exam-import-save'),
          onPressed: () {
            _saveSelection();
            Navigator.pop(context, true);
          },
          icon: const Icon(Icons.save_outlined),
          label: const Text('保存到模拟练习'),
        ),
      ],
    );
  }
}
