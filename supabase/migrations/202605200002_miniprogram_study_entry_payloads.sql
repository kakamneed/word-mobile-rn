create table if not exists public.study_entry_payloads (
  entry_id bigint primary key,
  source_id text not null unique,
  word text not null,
  part_of_speech text,
  frequency double precision not null default 0,
  phonetic_us text,
  phonetic_uk text,
  meanings_json jsonb not null default '[]'::jsonb,
  meaning_details_json jsonb not null default '[]'::jsonb,
  example_sentence text,
  example_translation text,
  wordbook_id bigint,
  rank_in_book integer not null default 0,
  is_active boolean not null default true,
  updated_at timestamptz not null default now()
);

create index if not exists study_entry_payloads_wordbook_rank_idx
  on public.study_entry_payloads (wordbook_id, rank_in_book)
  where is_active;

create index if not exists study_entry_payloads_active_frequency_idx
  on public.study_entry_payloads (is_active, frequency desc);

drop trigger if exists study_entry_payloads_set_updated_at
  on public.study_entry_payloads;

create trigger study_entry_payloads_set_updated_at
before update on public.study_entry_payloads
for each row execute function public.set_updated_at();

alter table public.study_entry_payloads enable row level security;
