import { invoke } from '@tauri-apps/api/core';
import type { TimerState } from '$lib/types';

export const timerStart    = () => invoke<TimerState>('timer_start');
export const timerPause    = () => invoke<TimerState>('timer_pause');
export const timerReset    = () => invoke<TimerState>('timer_reset');
export const timerSkip     = () => invoke<TimerState>('timer_skip');
export const timerGetState = () => invoke<TimerState>('timer_get_state');
