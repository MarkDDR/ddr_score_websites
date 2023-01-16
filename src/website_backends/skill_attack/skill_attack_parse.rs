use nom::{bytes::complete::tag, IResult};

use super::SkillAttackIndex;

struct ErrorTodo;

fn array_contents(input: &[u8]) -> IResult<&[u8], &[u8]> {}

fn num_literal_array(input: &[u8]) -> IResult<&[u8], Vec<SkillAttackIndex>> {
    let (input, _) = tag(b"new Array(")(input)?;
}
