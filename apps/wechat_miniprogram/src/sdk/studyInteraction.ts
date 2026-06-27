export function shouldSubmitChoiceTap(input: {
  questionId: string;
  choice: string;
  selected?: string;
  now: number;
  previous?: { questionId: string; value: string; at: number; submitted?: boolean };
}) {
  return Boolean(
    input.previous &&
      input.previous.questionId === input.questionId &&
      input.previous.value === input.choice &&
      input.now - input.previous.at < 420 &&
      input.previous.submitted !== true,
  );
}

export function handleStudyChoiceTap({
  questionId,
  choice,
  selected,
  now,
  previous,
  onSelect,
  onSubmit,
}: {
  questionId: string;
  choice: string;
  selected: string;
  now: number;
  previous?: { questionId: string; value: string; at: number; submitted?: boolean };
  onSelect: (value: string) => void;
  onSubmit: (value: string) => void;
}) {
  const shouldSubmit = shouldSubmitChoiceTap({ questionId, choice, selected, now, previous });
  if (shouldSubmit) {
    onSubmit(choice);
    return { selected: choice, shouldSubmit };
  }
  onSelect(choice);
  return { selected: choice, shouldSubmit };
}
