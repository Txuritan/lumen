<script lang="ts">
    import { state } from "../data";

    export let name: string

    const submit = async () => {
        let res = await fetch(`/api/character/remove/${encodeURI(name)}`)
        if (res.status == 200) {
            $state = await res.json()
        }
    }
</script>

<li>{name} <form method="get" on:submit|preventDefault={submit}><input type="submit" value="remove"></form></li>
