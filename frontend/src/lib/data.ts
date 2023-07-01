import * as store from "svelte/store"

export interface Stat {
    name: string;
    type: string;
}

export interface CharacterStat {
    name: string;
    type: string;
    value: boolean | number | string;
}

export interface Character {
    name: string;
    stats: CharacterStat[];
}

export interface Part {
    name: string;
    details: string;
    type: string;
    rarity: string;
}

interface Weapon {
    level: number;
    id: number;
    name: string;
    rarity: string;
    type: string;
    company: string;
    barrel: Part;
    body: Part;
    magazine: Part;
    stock: Part;
    range: string;
    damage: string;
    details: string[];
}

interface State {
    stats: Stat[];
    characters: Character[];
    parts: Part[];
    weapon?: Weapon;
}

export const state: store.Writable<State> = store.writable({
    stats: [],
    characters: [],
    parts: [],
    weapon: null,
});
