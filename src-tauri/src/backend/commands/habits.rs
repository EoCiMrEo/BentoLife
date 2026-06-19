use super::shared::*;

#[tauri::command]
pub fn read_habits(
    vault_path: String,
    summary_date: Option<String>,
) -> Result<HabitDocument, String> {
    HabitsService::read_habits(&checked_vault_path(vault_path)?, summary_date)
}

#[tauri::command]
pub fn create_habit(
    vault_path: String,
    habit: HabitInput,
    summary_date: Option<String>,
) -> Result<HabitDocument, String> {
    HabitsService::create_habit(&checked_vault_path(vault_path)?, habit, summary_date)
}

#[tauri::command]
pub fn update_habit(
    vault_path: String,
    habit_id: String,
    habit: HabitInput,
    summary_date: Option<String>,
) -> Result<HabitDocument, String> {
    HabitsService::update_habit(
        &checked_vault_path(vault_path)?,
        &habit_id,
        habit,
        summary_date,
    )
}

#[tauri::command]
pub fn record_habit_checkin(
    vault_path: String,
    habit_id: String,
    date: String,
) -> Result<HabitDocument, String> {
    HabitsService::record_checkin(&checked_vault_path(vault_path)?, &habit_id, &date)
}
