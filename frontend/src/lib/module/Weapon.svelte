<script lang="ts">
    import Fieldset from '../component/Fieldset.svelte'
    import { state } from '../data'

    let id = ""
    let level = 1

    const submitBuild = async () => {
        let res = await fetch("/api/weapon/build?" + new URLSearchParams({ id: id }))
        if (res.status == 200) {
            $state = await res.json()
            id = ""
        }
    }

    const submitGet = async () => {
        let res = await fetch("/api/weapon/generate?" + new URLSearchParams({ level: level.toString() }))
        if (res.status == 200) {
            $state = await res.json()
            level = 1
        }
    }
</script>

<Fieldset name={"Weapon"}>
    {#if $state.weapon}
    <p>lvl {$state.weapon.level} {$state.weapon.rarity} {$state.weapon.type}</p>
    <p><b>{$state.weapon.name}</b></p>
    <p><b>{$state.weapon.company}</b></p>
    <table>
        <tr><td>damage</td><td class="text-right">{$state.weapon.damage}</td></tr>
        <tr><td>range</td><td class="text-right">{$state.weapon.range}</td></tr>
    </table>
    <ul>
        {#each $state.weapon.details as detail}
        <li>{detail}</li>
        {/each}
    </ul>
    <var>{$state.weapon.id}</var>
    {/if}
    <hr>
    <form method="get" on:submit|preventDefault={submitBuild}>
        <input type="submit" value="build">
        <input type="text" name="id" placeholder="000:000:000:000:000:000" size="23" bind:value={id}>
    </form>
    <br>
    <form method="get" on:submit|preventDefault={submitGet}>
        <input type="submit" value="generate">
        <input type="number" name="level" size="8" bind:value={level}>
    </form>
</Fieldset>
