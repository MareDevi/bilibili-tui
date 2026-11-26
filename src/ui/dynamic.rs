//! Dynamic feed page with video card grid display

use super::video_card::{VideoCard, VideoCardGrid};
use super::{Component, Theme, shortcut_footer};
use crate::api::client::ApiClient;
use crate::api::dynamic::DynamicItem;
use crate::application::AppAction;
use crate::storage::Keybindings;
use ratatui::{
    crossterm::event::{KeyCode, MouseButton, MouseEvent},
    prelude::*,
    widgets::*,
};
use std::collections::HashMap;
use std::time::Instant;

/// Dynamic feed tab types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicTab {
    /// All dynamics (视频+图文)
    All,
    /// Video dynamics only
    Videos,
    /// Image/Opus dynamics (图文动态)
    Images,
}

impl DynamicTab {
    pub fn label(&self) -> &str {
        match self {
            DynamicTab::All => "全部",
            DynamicTab::Videos => "视频",
            DynamicTab::Images => "图文",
        }
    }

    pub fn all_tabs() -> [DynamicTab; 3] {
        [DynamicTab::All, DynamicTab::Videos, DynamicTab::Images]
    }

    /// Get the API feed type parameter for this tab
    pub fn get_feed_type(&self) -> Option<&str> {
        match self {
            DynamicTab::All => None, // No type filter = all types
            DynamicTab::Videos => Some("video"),
            DynamicTab::Images => Some("draw"), // draw type includes both draw and opus
        }
    }
}

pub struct DynamicPage {
    pub grid: VideoCardGrid,
    pub loading: bool,
    pub error_message: Option<String>,
    pub offset: Option<String>,
    pub has_more: bool,
    pub loading_more: bool,
    pub current_tab: DynamicTab,
    pub tab_offsets: HashMap<DynamicTab, Option<String>>,
    pub up_list: Vec<crate::api::dynamic::UpListItem>,
    pub selected_up_index: usize,
    pub focus_up_list: bool,
    pub loading_up_list: bool,
    pub up_list_scroll_offset: usize,
    pub dynamic_items: Vec<DynamicItem>,
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
}

impl DynamicPage {
    pub fn new() -> Self {
        Self {
            grid: VideoCardGrid::new_list(),
            loading: true,
            error_message: None,
            offset: None,
            has_more: false,
            loading_more: false,
            current_tab: DynamicTab::All,
            tab_offsets: HashMap::new(),
            up_list: Vec::new(),
            selected_up_index: 0,
            focus_up_list: true,
            loading_up_list: false,
            up_list_scroll_offset: 0,
            dynamic_items: Vec::new(),
            last_click_time: None,
            last_click_index: None,
        }
    }

    pub fn set_up_list(&mut self, up_list: Vec<crate::api::dynamic::UpListItem>) {
        self.up_list = up_list;
        self.loading_up_list = false;
    }

    pub fn select_up(&mut self, index: usize) {
        if index <= self.up_list.len() {
            self.selected_up_index = index;
            self.update_up_scroll();
            self.grid.clear();
            self.loading = true;
        }
    }

    /// Update scroll offset to keep selected UP visible
    fn update_up_scroll(&mut self) {
        const VISIBLE_UPS: usize = 10;
        // selected_up_index 0 is "全部", so actual UP indices start from 1
        // up_list_scroll_offset is the first UP index (1-based) to show after "全部"
        if self.selected_up_index == 0 {
            // "全部" is always visible, scroll to beginning
            self.up_list_scroll_offset = 0;
        } else {
            // Ensure selected UP is within visible range
            let effective_idx = self.selected_up_index; // 1-based index into up_list
            if effective_idx <= self.up_list_scroll_offset {
                // Selected is before visible range, scroll left
                self.up_list_scroll_offset = effective_idx.saturating_sub(1);
            } else if effective_idx > self.up_list_scroll_offset + VISIBLE_UPS {
                // Selected is after visible range, scroll right
                self.up_list_scroll_offset = effective_idx.saturating_sub(VISIBLE_UPS);
            }
        }
    }

    pub fn get_selected_up_mid(&self) -> Option<i64> {
        if self.selected_up_index == 0 {
            None
        } else {
            self.up_list.get(self.selected_up_index - 1).map(|u| u.mid)
        }
    }

    pub fn switch_tab(&mut self, tab: DynamicTab) {
        if self.current_tab != tab {
            self.current_tab = tab;
            self.offset = self.tab_offsets.get(&tab).cloned().flatten();
            self.grid.clear();
            self.loading = true;
            self.error_message = None;
        }
    }

    pub fn set_feed(&mut self, items: Vec<DynamicItem>, offset: Option<String>, has_more: bool) {
        self.grid.clear();
        self.dynamic_items.clear();

        // Process items based on current tab filter
        for item in items.into_iter() {
            let should_include = match self.current_tab {
                DynamicTab::All => item.is_video() || item.is_draw() || item.is_opus(),
                DynamicTab::Videos => item.is_video(),
                DynamicTab::Images => item.is_draw() || item.is_opus(),
            };

            if !should_include {
                continue;
            }

            // Store the item
            self.dynamic_items.push(item.clone());

            // Handle video dynamics
            if item.is_video() {
                if let Some(bvid) = item.video_bvid() {
                    let card = VideoCard::new(
                        Some(bvid.to_string()),
                        item.video_aid(),
                        item.video_title().unwrap_or("无标题").to_string(),
                        item.author_name().to_string(),
                        format!("▶ {}", item.video_play()),
                        item.video_duration().to_string(),
                        item.video_cover().map(|s| s.to_string()),
                    );
                    self.grid.add_card(card);
                }
            }
            // Handle image dynamics (带图动态)
            else if item.is_draw() {
                let images = item.draw_images();
                let image_url = images.first().map(|s| s.to_string());
                let desc = item.desc_text().unwrap_or("图片动态");
                let image_count = if images.len() > 1 {
                    format!(" [{}P]", images.len())
                } else {
                    String::new()
                };

                let card = VideoCard::new(
                    None, // No bvid for images
                    None,
                    format!("{}{}", desc, image_count),
                    item.author_name().to_string(),
                    "📷 图片动态".to_string(),
                    "".to_string(),
                    image_url,
                );
                self.grid.add_card(card);
            }
            // Handle text/opus dynamics (图文动态)
            else if item.is_opus() {
                let text = item.opus_text().unwrap_or("图文动态");
                let images = item.opus_images();
                let image_url = images.first().map(|s| s.to_string());
                let image_count = if !images.is_empty() {
                    format!(" [{}P]", images.len())
                } else {
                    String::new()
                };

                let card = VideoCard::new(
                    None,
                    None,
                    format!("{}{}", text, image_count),
                    item.author_name().to_string(),
                    "📝 图文".to_string(),
                    "".to_string(),
                    image_url,
                );
                self.grid.add_card(card);
            }
        }

        // Save offset for current tab
        self.tab_offsets.insert(self.current_tab, offset.clone());
        self.offset = offset;
        self.has_more = has_more;
        self.loading = false;
    }

    pub fn append_feed(&mut self, items: Vec<DynamicItem>, offset: Option<String>, has_more: bool) {
        // Process items based on current tab filter
        for item in items.into_iter() {
            let should_include = match self.current_tab {
                DynamicTab::All => item.is_video() || item.is_draw() || item.is_opus(),
                DynamicTab::Videos => item.is_video(),
                DynamicTab::Images => item.is_draw() || item.is_opus(),
            };

            if !should_include {
                continue;
            }

            // Store the item
            self.dynamic_items.push(item.clone());

            // Handle video dynamics
            if item.is_video() {
                if let Some(bvid) = item.video_bvid() {
                    let card = VideoCard::new(
                        Some(bvid.to_string()),
                        item.video_aid(),
                        item.video_title().unwrap_or("无标题").to_string(),
                        item.author_name().to_string(),
                        format!("▶ {}", item.video_play()),
                        item.video_duration().to_string(),
                        item.video_cover().map(|s| s.to_string()),
                    );
                    self.grid.add_card(card);
                }
            }
            // Handle image dynamics
            else if item.is_draw() {
                let images = item.draw_images();
                let image_url = images.first().map(|s| s.to_string());
                let desc = item.desc_text().unwrap_or("图片动态");
                let image_count = if images.len() > 1 {
                    format!(" [{}P]", images.len())
                } else {
                    String::new()
                };

                let card = VideoCard::new(
                    None,
                    None,
                    format!("{}{}", desc, image_count),
                    item.author_name().to_string(),
                    "📷 图片动态".to_string(),
                    "".to_string(),
                    image_url,
                );
                self.grid.add_card(card);
            }
            // Handle text/opus dynamics
            else if item.is_opus() {
                let text = item.opus_text().unwrap_or("图文动态");
                let images = item.opus_images();
                let image_url = images.first().map(|s| s.to_string());
                let image_count = if !images.is_empty() {
                    format!(" [{}P]", images.len())
                } else {
                    String::new()
                };

                let card = VideoCard::new(
                    None,
                    None,
                    format!("{}{}", text, image_count),
                    item.author_name().to_string(),
                    "📝 图文".to_string(),
                    "".to_string(),
                    image_url,
                );
                self.grid.add_card(card);
            }
        }

        // Save offset for current tab
        self.tab_offsets.insert(self.current_tab, offset.clone());
        self.offset = offset;
        self.has_more = has_more;
        self.loading_more = false;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.loading = false;
        self.loading_more = false;
    }

    pub async fn load_more(&mut self, api_client: &ApiClient) {
        if self.loading_more || !self.has_more {
            return;
        }

        self.loading_more = true;

        let feed_type = self.current_tab.get_feed_type();
        let host_mid = self.get_selected_up_mid();
        match api_client
            .get_dynamic_feed(self.offset.as_deref(), feed_type, host_mid)
            .await
        {
            Ok(data) => {
                let items = data.items.unwrap_or_default();
                let offset = data.offset;
                let has_more = data.has_more.unwrap_or(false);
                self.append_feed(items, offset, has_more);
            }
            Err(_) => {
                self.loading_more = false;
            }
        }
    }

    pub fn poll_cover_results(&mut self) {
        self.grid.poll_cover_results();
    }

    pub fn start_cover_downloads(&mut self) {
        self.grid.start_cover_downloads();
    }

    /// Get the currently selected dynamic item (if any)
    pub fn selected_dynamic_item(&self) -> Option<&DynamicItem> {
        let selected_index = self.grid.selected_index;
        self.dynamic_items.get(selected_index)
    }
}

impl Default for DynamicPage {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicPage {
    fn draw_up_list(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut items = vec![ListItem::new("全部")];
        items.extend(self.up_list.iter().map(|user| {
            let marker = if user.has_update { "● " } else { "  " };
            ListItem::new(format!("{marker}{}", user.uname))
        }));
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border_subtle))
                    .title(" 关注的UP主 "),
            )
            .highlight_symbol("▶ ")
            .highlight_style(Style::default().fg(if self.focus_up_list {
                theme.bilibili_pink
            } else {
                theme.bilibili_cyan
            }));
        let mut state = ListState::default().with_selected(Some(self.selected_up_index));
        frame.render_stateful_widget(list, area, &mut state);
    }
}

impl Component for DynamicPage {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, keys: &Keybindings) {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        self.draw_up_list(frame, panes[0], theme);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(panes[1]);
        let header_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(3)])
            .split(chunks[0]);
        let title = Paragraph::new(Line::from(vec![
            Span::styled(" 📺 ", Style::default()),
            Span::styled(
                "关注动态",
                Style::default()
                    .fg(theme.bilibili_pink)
                    .add_modifier(Modifier::BOLD),
            ),
            if self.loading_more {
                Span::styled(" 加载中...", Style::default().fg(theme.warning))
            } else {
                Span::raw("")
            },
        ]))
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));
        frame.render_widget(title, header_chunks[0]);

        let tabs = DynamicTab::all_tabs()
            .iter()
            .enumerate()
            .flat_map(|(index, tab)| {
                let prefix = (index > 0).then_some(Span::raw("  "));
                let style = if *tab == self.current_tab {
                    Style::default()
                        .fg(theme.fg_accent)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.fg_secondary)
                };
                [
                    prefix,
                    Some(Span::styled(
                        format!("[{}] {}", index + 1, tab.label()),
                        style,
                    )),
                ]
                .into_iter()
                .flatten()
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(Line::from(tabs))
                .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT))
                .alignment(Alignment::Center),
            header_chunks[1],
        );

        if self.loading {
            frame.render_widget(
                Paragraph::new("⏳ 加载动态中...")
                    .style(Style::default().fg(theme.warning))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else if let Some(error) = &self.error_message {
            frame.render_widget(
                Paragraph::new(format!("❌ {error}"))
                    .style(Style::default().fg(theme.error))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else if self.grid.cards.is_empty() {
            frame.render_widget(
                Paragraph::new("暂无动态，请先登录并关注UP主")
                    .style(Style::default().fg(theme.fg_secondary))
                    .alignment(Alignment::Center),
                chunks[1],
            );
        } else {
            self.grid.render(frame, chunks[1], theme);
        }

        frame.render_widget(
            Paragraph::new(shortcut_footer(
                theme,
                [
                    ("↑/↓".into(), "选择动态".into(), theme.fg_accent),
                    (
                        format!("{}/{}", keys.page_up, keys.page_down),
                        "翻页".into(),
                        theme.fg_accent,
                    ),
                    ("←/→".into(), "切换面板".into(), theme.fg_accent),
                    (keys.tab_1.clone(), "切标签".into(), theme.info),
                    (keys.confirm.clone(), "详情".into(), theme.success),
                    (keys.refresh.clone(), "刷新".into(), theme.info),
                ],
            ))
            .alignment(Alignment::Center),
            chunks[2],
        );
    }

    fn handle_input_with_modifiers(
        &mut self,
        key: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
        keys: &crate::storage::Keybindings,
    ) -> Option<AppAction> {
        let _ = modifiers;
        if keys.matches_quit(key) {
            return Some(AppAction::Quit);
        }
        if keys.matches_nav_next(key) {
            return Some(AppAction::NavNext);
        }
        if keys.matches_nav_prev(key) {
            return Some(AppAction::NavPrev);
        }

        if self.focus_up_list {
            if keys.matches_down(key) {
                if self.selected_up_index < self.up_list.len() {
                    return Some(AppAction::SelectUpMaster(self.selected_up_index + 1));
                }
            } else if keys.matches_up(key) {
                if self.selected_up_index > 0 {
                    return Some(AppAction::SelectUpMaster(self.selected_up_index - 1));
                }
            } else if keys.matches_right(key) || keys.matches_confirm(key) {
                self.focus_up_list = false;
            }
            return Some(AppAction::None);
        }

        if keys.matches_left(key) {
            self.focus_up_list = true;
            if self.loading || self.loading_more {
                self.loading = false;
                self.loading_more = false;
                return Some(AppAction::CancelPendingLoads);
            }
            return Some(AppAction::None);
        }

        if keys.matches_page_down(key) {
            self.grid.move_page_down();
            if self.grid.is_near_bottom(self.grid.cached_visible_rows)
                && !self.loading_more
                && self.has_more
            {
                return Some(AppAction::LoadMoreDynamic);
            }
            return Some(AppAction::None);
        }
        if keys.matches_page_up(key) {
            self.grid.move_page_up();
            return Some(AppAction::None);
        }

        if keys.matches_down(key) {
            self.grid.move_down();
            if self.grid.is_near_bottom(3) && !self.loading_more && self.has_more {
                return Some(AppAction::LoadMoreDynamic);
            }
            return Some(AppAction::None);
        }
        if keys.matches_up(key) {
            self.grid.move_up();
            return Some(AppAction::None);
        }

        // Direct tab access
        if keys.matches_tab_1(key) {
            return Some(AppAction::SwitchDynamicTab(DynamicTab::All));
        }
        if keys.matches_tab_2(key) {
            return Some(AppAction::SwitchDynamicTab(DynamicTab::Videos));
        }
        if keys.matches_tab_3(key) {
            return Some(AppAction::SwitchDynamicTab(DynamicTab::Images));
        }

        // Open selected card
        if key == KeyCode::Char('u')
            && let Some(mid) = self
                .selected_dynamic_item()
                .and_then(|item| item.author_mid())
        {
            return Some(AppAction::OpenUpPage(mid));
        }
        if keys.matches_confirm(key) {
            if let Some(card) = self.grid.selected_card() {
                // Video card - open video detail
                if let Some(ref bvid) = card.bvid {
                    return Some(AppAction::OpenVideoDetail(
                        bvid.clone(),
                        card.aid.unwrap_or(0),
                    ));
                }
                // Non-video card (draw/opus) - open dynamic detail
                else if let Some(item) = self.selected_dynamic_item()
                    && (item.is_draw() || item.is_opus())
                    && let Some(id) = &item.id_str
                {
                    return Some(AppAction::OpenDynamicDetail(id.clone()));
                }
            }
            return Some(AppAction::None);
        }

        // Refresh
        if keys.matches_refresh(key) {
            self.loading = true;
            self.grid.clear();
            return Some(AppAction::RefreshDynamic);
        }

        Some(AppAction::None)
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> Option<AppAction> {
        use crossterm::event::MouseEventKind;
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(28), Constraint::Min(30)])
            .split(area);
        let position = ratatui::layout::Position::new(event.column, event.row);

        if panes[0].contains(position) {
            self.focus_up_list = true;
            match event.kind {
                MouseEventKind::ScrollDown if self.selected_up_index < self.up_list.len() => {
                    return Some(AppAction::SelectUpMaster(self.selected_up_index + 1));
                }
                MouseEventKind::ScrollUp if self.selected_up_index > 0 => {
                    return Some(AppAction::SelectUpMaster(self.selected_up_index - 1));
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let row = event.row.saturating_sub(panes[0].y + 1) as usize;
                    if row <= self.up_list.len() {
                        self.focus_up_list = true;
                        if row != self.selected_up_index {
                            return Some(AppAction::SelectUpMaster(row));
                        }
                    }
                }
                _ => {}
            }
            return Some(AppAction::None);
        }
        if !panes[1].contains(position) {
            return None;
        }
        self.focus_up_list = false;

        match event.kind {
            MouseEventKind::ScrollDown => {
                if self.grid.move_down()
                    && self.grid.is_near_bottom(3)
                    && !self.loading_more
                    && self.has_more
                {
                    return Some(AppAction::LoadMoreDynamic);
                }
                None
            }
            MouseEventKind::ScrollUp => {
                self.grid.move_up();
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5),
                        Constraint::Min(10),
                        Constraint::Length(2),
                    ])
                    .split(panes[1]);

                let grid_area = chunks[1];

                if !grid_area.contains(ratatui::layout::Position::new(event.column, event.row)) {
                    return None;
                }

                let relative_y = event.row - grid_area.y;
                let click_row = (relative_y / self.grid.card_height) as usize;
                let actual_row = self.grid.scroll_row + click_row;

                let card_width = grid_area.width / self.grid.columns as u16;
                let click_col = (event.column.saturating_sub(grid_area.x) / card_width) as usize;

                let click_idx = actual_row * self.grid.columns + click_col;

                if click_idx < self.grid.cards.len() {
                    let now = Instant::now();
                    let is_double_click = self.last_click_index == Some(click_idx)
                        && self
                            .last_click_time
                            .is_some_and(|t| now.duration_since(t).as_millis() < 500);

                    if is_double_click {
                        self.last_click_time = None;
                        self.last_click_index = None;
                        if let Some(card) = self.grid.cards.get(click_idx) {
                            if let Some(ref bvid) = card.bvid {
                                return Some(AppAction::OpenVideoDetail(
                                    bvid.clone(),
                                    card.aid.unwrap_or(0),
                                ));
                            } else if let Some(item) = self.dynamic_items.get(click_idx)
                                && (item.is_draw() || item.is_opus())
                                && let Some(id) = &item.id_str
                            {
                                return Some(AppAction::OpenDynamicDetail(id.clone()));
                            }
                        }
                    } else {
                        self.grid.selected_index = click_idx;
                        self.grid.update_scroll(self.grid.cached_visible_rows);
                        self.last_click_time = Some(now);
                        self.last_click_index = Some(click_idx);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_uses_sidebar_up_selection_and_single_column_cards() {
        let page = DynamicPage::new();
        assert!(page.focus_up_list);
        assert_eq!(page.grid.columns, 1);
        assert!(page.grid.list_layout);
    }

    #[test]
    fn dynamic_right_switches_to_the_card_pane() {
        let mut page = DynamicPage::new();
        let keys = Keybindings::default();
        assert!(matches!(
            page.handle_input_with_modifiers(
                KeyCode::Right,
                crossterm::event::KeyModifiers::NONE,
                &keys
            ),
            Some(AppAction::None)
        ));
        assert!(!page.focus_up_list);
    }

    fn video_dynamic(archive_aid: &str) -> DynamicItem {
        serde_json::from_str(&format!(
            r#"{{
              "id_str": "9001",
              "type": "DYNAMIC_TYPE_AV",
              "modules": {{"module_dynamic": {{"major": {{
                "type": "MAJOR_TYPE_ARCHIVE",
                "archive": {{"bvid": "BV1test", "title": "t", "aid": {archive_aid}}}
              }}}}
              }}
            }}"#
        ))
        .expect("video dynamic")
    }

    fn confirm_action(page: &mut DynamicPage) -> Option<AppAction> {
        page.focus_up_list = false;
        page.handle_input_with_modifiers(
            KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
            &Keybindings::default(),
        )
    }

    #[test]
    fn set_feed_and_append_feed_preserve_video_aid() {
        let mut page = DynamicPage::new();
        page.set_feed(vec![video_dynamic("\"123\"")], None, false);
        assert_eq!(page.grid.cards.first().and_then(|card| card.aid), Some(123));
        page.append_feed(vec![video_dynamic("456")], None, false);
        assert_eq!(page.grid.cards.get(1).and_then(|card| card.aid), Some(456));
    }

    #[test]
    fn confirm_video_card_opens_detail_with_card_aid() {
        let mut page = DynamicPage::new();
        page.set_feed(vec![video_dynamic("\"123\"")], None, false);
        assert!(matches!(
            confirm_action(&mut page),
            Some(AppAction::OpenVideoDetail(bvid, 123)) if bvid == "BV1test"
        ));
    }

    #[test]
    fn confirm_video_card_without_aid_opens_detail_with_zero() {
        let mut page = DynamicPage::new();
        page.set_feed(vec![video_dynamic("null")], None, false);
        assert!(matches!(
            confirm_action(&mut page),
            Some(AppAction::OpenVideoDetail(bvid, 0)) if bvid == "BV1test"
        ));
    }
}
