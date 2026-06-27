export class MemoryStudyDomain {
  constructor({ adapter }) {
    this.adapter = adapter;
  }

  startStudySession(input) {
    return this.adapter.startStudySession(input);
  }

  submitStudyAnswer(input) {
    return this.adapter.submitStudyAnswer(input);
  }

  getResumeSessionHint(input) {
    return this.adapter.getResumeSessionHint(input);
  }

  listWrongWords(input) {
    return this.adapter.listWrongWords(input);
  }

  getWrongWordDetail(input) {
    return this.adapter.getWrongWordDetail(input);
  }

  completeStudySession(input) {
    return this.adapter.completeStudySession(input);
  }

  cancelStudySession(input) {
    return this.adapter.cancelStudySession(input);
  }

  markStudyEntryMastered(input) {
    return this.adapter.markStudyEntryMastered(input);
  }

  acceptDisputedMeaning(input) {
    return this.adapter.acceptDisputedMeaning(input);
  }
}
