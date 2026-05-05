alter table public.wrong_word_entries
  add column if not exists hint_text text not null default '',
  add column if not exists hint_source text not null default '',
  add column if not exists hint_updated_at timestamptz;

drop trigger if exists wrong_word_entries_set_updated_at on public.wrong_word_entries;

create trigger wrong_word_entries_set_updated_at
before update on public.wrong_word_entries
for each row execute function public.set_updated_at();
