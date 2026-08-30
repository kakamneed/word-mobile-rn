import 'package:flutter/foundation.dart';

import '../sdk/exam_practice_client.dart';

class ExamAnalysisTaskNotifications {
  ExamAnalysisTaskNotifications._();

  static final ValueNotifier<int> unreadCount = ValueNotifier<int>(0);

  static Future<ExamAnalysisInbox> refresh(ExamPracticeClient client) async {
    final inbox = await client.getAnalysisTasks();
    unreadCount.value = inbox.unreadCount;
    return inbox;
  }
}
