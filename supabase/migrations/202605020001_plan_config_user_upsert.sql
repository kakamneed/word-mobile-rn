create unique index if not exists plan_configs_user_id_unique
on public.plan_configs(user_id);

notify pgrst, 'reload schema';
