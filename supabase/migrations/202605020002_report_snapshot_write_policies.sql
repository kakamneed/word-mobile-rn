drop policy if exists report_snapshots_owner_insert on public.report_snapshots;
drop policy if exists report_snapshots_owner_update on public.report_snapshots;

create policy report_snapshots_owner_insert on public.report_snapshots
for insert with check (auth.uid() = user_id);

create policy report_snapshots_owner_update on public.report_snapshots
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

notify pgrst, 'reload schema';
