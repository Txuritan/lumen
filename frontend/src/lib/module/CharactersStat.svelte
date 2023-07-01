<script lang="ts">
    import { state, type Character, type CharacterStat } from "../data"

    export let character: Character
    export let stat: CharacterStat

    const submitIncrement = async () => {
        let res = await fetch(`/api/character/increment/${encodeURI(character.name)}/${encodeURI(stat.name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }

    const submitDecrement = async () => {
        let res = await fetch(`/api/character/decrement/${encodeURI(character.name)}/${encodeURI(stat.name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }

    const submitToggle = async () => {
        let res = await fetch(`/api/character/toggle/${encodeURI(character.name)}/${encodeURI(stat.name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<tr>
    <td><var>{stat.name}</var></td>
    <td class="text-right">
        {#if stat.type == "number"}
        <form method="get" on:submit|preventDefault={submitIncrement}><input type="submit" value="+"></form>{stat.value}<form method="get" on:submit|preventDefault={submitDecrement}><input type="submit" value="-"></form>
        {:else if stat.type == "boolean"}
        <form method="get" on:submit|preventDefault={submitToggle}><input type="checkbox" name="toggle" value="{stat.value}" on:change|preventDefault={submitToggle}></form>
        {/if}
    </td>
</tr>