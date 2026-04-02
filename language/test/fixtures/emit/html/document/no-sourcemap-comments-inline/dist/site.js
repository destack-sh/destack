export const padZero = (num) => String(num).padStart(2, "0");

export const formatDate = (
    date,
) => `${date.getFullYear()}-${padZero(date.getMonth() + 1)}-${padZero(date.getDate())}`;

export const greeting = (name) => `Hello, ${name}!`;

export function renderSummary(name) {
    return `${greeting(name)}:${padZero(7)}`;
}

console.log(greeting("World"));
console.log(formatDate(new Date()));
console.log(renderSummary("World"));
//# sourceMappingURL=data:application/json;charset=utf-8;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbInNyYy9pbi91dGlscy9udW1iZXJzLnRzIiwic3JjL2luL3V0aWxzL2RhdGUudHMiLCJzcmMvaW4vdXRpbHMvc3RyaW5ncy50cyIsInNyYy9pbi91dGlscy9zdW1tYXJ5LnRzIiwic3JjL2luL21haW4udHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQUEsYUFBYSxPQUFPLEdBQUcsQ0FBQyxHQUFHLFlBQVksR0FBRyxXQUFXLENBQUMsRUFBRSxHQUFHLENBQUM7O0FDRTVELGFBQWEsVUFBVSxHQUFHO0lBQUMsSUFBSTtRQUN4QixrQkFBa0IsSUFBSSxRQUFRLG1CQUFtQixDQUFDLElBQUksUUFBUSxjQUFjLENBQUMsRUFBRTs7QUNIdEYsYUFBYSxRQUFRLEdBQUcsQ0FBQyxJQUFJLGVBQWUsSUFBSSxHQUFHOztBQ0duRCw4QkFBOEIsSUFBWSxFQUFFO0lBQ3hDLE9BQU8sR0FBRyxTQUFTLElBQUksQ0FBQyxJQUFJLFFBQVEsQ0FBQyxDQUFDLEVBQUU7Q0FDM0M7O0FDREQsWUFBWSxTQUFTLE9BQU8sQ0FBQyxDQUFFO0FBQy9CLFlBQVksV0FBVyxVQUFVLENBQUMsQ0FBRTtBQUNwQyxZQUFZLGNBQWMsT0FBTyxDQUFDLENBQUU7In0=
